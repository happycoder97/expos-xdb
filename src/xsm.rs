use rexpect;
use rexpect::session::PtySession;
use rexpect::ReadUntil;

use std::fs;

const XSM_PAGE_LEN: usize = 512;

pub struct XSM {
    xsm: PtySession,
    mode: Mode,
    regs: XSMRegs,
    page_table: Vec<XSMPageTableEntry>,
    errors: Vec<XSMError>,
    output: String,
    halted: bool,
}

#[derive(Debug)]
pub enum XSMError {
    PTBRInvalid,
    PTLRInvalid,
    InvalidPageTableEntry {
        index: usize,
        entry: XSMPageTableEntry,
    },
}

#[derive(Debug)]
enum XSMInternalError {
    InvalidPageTableEntry {
        index: usize,
        entry: XSMPageTableEntry,
    },
    VirtualMemoryNotPaged {
        page: usize,
        entry: XSMPageTableEntry,
    },
    VirtualMemoryOutOfBounds {
        addr: usize,
    },
    DebugModeNotEntered {
        lines_read: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct XSMPageTableEntry {
    phy: String,
    aux: String,
}

#[derive(Debug)]
pub enum Mode {
    Kernel,
    User,
}

#[derive(Debug)]
pub struct XSMRegs {
    r: [String; 20],
    p: [String; 4],
    bp: String,
    sp: String,
    ip: String,
    ptbr: String,
    ptlr: String,
    eip: String,
    ec: String,
    epn: String,
    ema: String,
}

impl Default for XSMRegs {
    fn default() -> Self {
        Self {
            r: Default::default(),
            p: Default::default(),
            bp: Default::default(),
            sp: Default::default(),
            ip: Default::default(),
            ptbr: Default::default(),
            ptlr: Default::default(),
            eip: Default::default(),
            ec: Default::default(),
            epn: Default::default(),
            ema: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum XSMSpawnError {
    CommandNotFound,
    FailedToEnterDebugMode { lines_read: Vec<String> },
}

impl XSM {
    pub fn spawn_new(command: &str) -> Result<XSM, XSMSpawnError> {
        let xsm_process =
            rexpect::spawn(command, Some(10)).map_err(|_| XSMSpawnError::CommandNotFound)?;
        let mut xsm = XSM {
            xsm: xsm_process,
            mode: Mode::Kernel,
            regs: XSMRegs::default(),
            page_table: Vec::default(),
            errors: Vec::default(),
            output: String::default(),
            halted: false,
        };
        xsm.load_state().map_err(|e| {
            if let XSMInternalError::DebugModeNotEntered { lines_read } = e {
                XSMSpawnError::FailedToEnterDebugMode { lines_read }
            } else {
                panic!("Invalid error")
            }
        })?;
        Ok(xsm)
    }

    /// Must be called right after entering debug mode
    /// or right after sending step command
    /// Returns: Mode, Program output if there was an out instruction
    fn load_state(&mut self) -> Result<(), XSMInternalError> {
        let t = std::time::Instant::now();
        let status = Self::_read_status(&mut self.xsm);
        eprintln!("Read status: {}", t.elapsed().as_millis());
        let t = std::time::Instant::now();
        self.xsm.exp_char('>').map_err(|_| {
            let mut v = Vec::new();
            while let Ok(line) = self.xsm.read_line() {
                v.push(line);
            }
            XSMInternalError::DebugModeNotEntered { lines_read: v }
        })?;
        eprintln!("Return to prompt: {}", t.elapsed().as_millis());
        let (mode, output) = status.expect("Failed to read status");
        self.mode = mode;
        self.output = output;
        let t = std::time::Instant::now();
        Self::_read_regs(&mut self.xsm, &mut self.regs);
        eprintln!("Read regs: {}", t.elapsed().as_millis());
        self.errors.clear();
        // let page_table =
        //     match Self::_read_page_table(&mut self.xsm, &self.regs.ptbr, &self.regs.ptlr) {
        //         Ok(page_table) => page_table,
        //         Err(e) => {
        //             self.errors.push(e);
        //             Vec::new()
        //         }
        //     };
        Ok(())
    }

    /// ------------ Called by load state --------------- ///
    fn _read_status(xsm: &mut PtySession) -> Result<(Mode, String), String> {
        let prog_out = xsm
            .exp_string("Previous instruction")
            .map_err(|_| String::from("Unable to read status"))?;
        xsm.exp_string("Mode: ")
            .map_err(|_| String::from("Unable to detect mode."))?;
        let mode_str = xsm
            .exp_any(vec![
                ReadUntil::String("KERNEL".into()),
                ReadUntil::String("USER".into()),
            ])
            .map_err(|_| String::from("Unexpected Mode value"))?
            .1;
        let mode = match mode_str.chars().nth(0).unwrap() {
            'K' => Mode::Kernel,
            'U' => Mode::User,
            _ => panic!(format!("Unexpected mode: {}", &mode_str)),
        };
        Ok((mode, prog_out))
    }

    fn _read_regs(xsm: &mut PtySession, regs: &mut XSMRegs) {
        xsm.send_line("reg")
            .expect("Couldn't send reg command to xsm");
        let mut lines = Vec::new();
        while let Ok(line) = xsm.read_line() {
            lines.push(line);
        }
        xsm.exp_char('>')
            .expect("reg: xsm didn't re-enter interactive debug mode.");

        fn ref_table(i: usize, regs: &mut XSMRegs) -> &mut String {
            if i < 20 {
                &mut regs.r[i]
            } else if (i - 20) < 4 {
                &mut regs.p[i - 20]
            } else {
                [
                    &mut regs.bp,
                    &mut regs.sp,
                    &mut regs.ip,
                    &mut regs.ptbr,
                    &mut regs.ptlr,
                    &mut regs.eip,
                    &mut regs.ec,
                    &mut regs.epn,
                    &mut regs.ema,
                ][i - 20 - 4]
            }
        }

        let mut i = 0;
        for line in lines.into_iter() {
            for word in line.split('\t') {
                if word == "" {
                    continue;
                }
                let mut it = word.split(": ");
                it.next().unwrap();
                let val = it.next().unwrap_or("").to_string();
                *ref_table(i, regs) = val;
                i += 1;
            }
        }
    }

    fn _read_page_table(
        xsm: &mut PtySession,
        ptbr: &str,
        ptlr: &str,
    ) -> Result<Vec<XSMPageTableEntry>, XSMError> {
        let mut page_table = Vec::new();
        let ptbr: usize = ptbr.parse().map_err(|_| XSMError::PTBRInvalid)?;
        let ptlr: usize = ptlr.parse().map_err(|_| XSMError::PTLRInvalid)?;
        let page_table_str = Self::read_mem_range(xsm, ptbr, ptbr + ptlr * 2 - 1);
        for entry_mem in page_table_str.chunks_exact(2) {
            let entry = XSMPageTableEntry {
                phy: entry_mem[0].clone(),
                aux: entry_mem[1].clone(),
            };
            page_table.push(entry);
        }
        Ok(page_table)
    }
    /// ------------ End of called by load state --------------- ///

    fn _pageify(start_addr: usize, end_addr: usize) -> (usize, usize, usize, usize) {
        let start_page = start_addr / XSM_PAGE_LEN;
        let start_page_skip = start_addr - start_page * XSM_PAGE_LEN;
        let end_page = end_addr / XSM_PAGE_LEN;
        let end_page_take = end_addr - end_page * XSM_PAGE_LEN;
        (start_page, end_page, start_page_skip, end_page_take)
    }

    fn _page_vir_to_phy(
        page_table: &Vec<XSMPageTableEntry>,
        vir_page: usize,
    ) -> Result<usize, XSMInternalError> {
        if vir_page > page_table.len() {
            return Err(XSMInternalError::VirtualMemoryOutOfBounds {
                addr: vir_page * XSM_PAGE_LEN,
            });
        }
        let page_table_entry = &page_table[vir_page];
        match page_table_entry.phy.parse::<isize>() {
            Err(_) => Err(XSMInternalError::InvalidPageTableEntry {
                index: vir_page,
                entry: page_table_entry.clone(),
            }),
            Ok(i) => {
                if i == -1 {
                    Err(XSMInternalError::VirtualMemoryNotPaged {
                        page: vir_page,
                        entry: page_table_entry.clone(),
                    })
                } else if i < -1 {
                    Err(XSMInternalError::InvalidPageTableEntry {
                        index: vir_page,
                        entry: page_table_entry.clone(),
                    })
                } else {
                    Ok(i as usize)
                }
            }
        }
    }

    fn read_mem_page(xsm: &mut PtySession, page: usize) -> Vec<String> {
        xsm.send_line(&format!("mem {}", page))
            .expect("Failed writing to xsm");
        // xsm.exp_string("Written to file mem.")
        //     .expect("Failed getting `mem` response from xsm.");
        xsm.exp_char('>')
            .expect("mem: xsm didn't re-enter interactive debug mode.");
        let mem: String = fs::read_to_string("mem").expect("Failed to read mem file.");
        mem.lines()
            .map(|l| {
                let mut s = l.split(": ");
                let _line_num = s.next();
                let content = s.next().unwrap();
                String::from(content)
            })
            .collect()
    }

    fn read_mem_range(xsm: &mut PtySession, start_addr: usize, end_addr: usize) -> Vec<String> {
        let mut data = Vec::new();
        let (start_page, end_page, start_page_skip, end_page_take) =
            Self::_pageify(start_addr, end_addr);
        data.extend(
            Self::read_mem_page(xsm, start_page)
                .into_iter()
                .skip(start_page_skip),
        );
        for i in start_page + 1..end_page {
            data.extend(Self::read_mem_page(xsm, i).into_iter());
        }
        if end_page > start_page {
            data.extend(
                Self::read_mem_page(xsm, end_page)
                    .into_iter()
                    .take(end_page_take),
            );
        }
        data
    }

    fn read_mem_range_vir(
        xsm: &mut PtySession,
        start_addr: usize,
        end_addr: usize,
        page_table: &Vec<XSMPageTableEntry>,
    ) -> Result<Vec<String>, XSMInternalError> {
        let mut data = Vec::new();
        let (start_page_vir, end_page_vir, start_page_skip, end_page_take) =
            Self::_pageify(start_addr, end_addr);
        let start_page_phy = Self::_page_vir_to_phy(&page_table, start_page_vir)?;
        data.extend(
            Self::read_mem_page(xsm, start_page_phy)
                .into_iter()
                .skip(start_page_skip),
        );
        for page_vir in start_page_vir + 1..end_page_vir {
            let page_phy = Self::_page_vir_to_phy(&page_table, page_vir)?;
            data.extend(Self::read_mem_page(xsm, page_phy).into_iter());
        }
        if end_page_vir > start_page_vir {
            let end_page_phy = Self::_page_vir_to_phy(&page_table, end_page_vir)?;
            data.extend(
                Self::read_mem_page(xsm, end_page_phy)
                    .into_iter()
                    .take(end_page_take),
            );
        }
        Ok(data)
    }

    fn get_valid_mem_range(
        include_addr: usize,
        page_table: &Vec<XSMPageTableEntry>,
    ) -> Option<(usize, usize)> {
        let page = include_addr / XSM_PAGE_LEN;
        let pt_entry: &XSMPageTableEntry = page_table.get(page)?;
        let _phy: usize = pt_entry.phy.parse().ok()?;

        let mut preceding_page = page;
        while preceding_page > 0 {
            let page_ = preceding_page - 1;
            let pt_entry = &page_table[page_];
            if pt_entry.phy.parse::<usize>().is_ok() {
                preceding_page = page_;
            } else {
                break;
            }
        }

        let mut succeeding_page = page;
        while succeeding_page < page_table.len() - 1 {
            let page_ = succeeding_page + 1;
            let pt_entry = &page_table[page_];
            if pt_entry.phy.parse::<usize>().is_ok() {
                succeeding_page = page_;
            } else {
                break;
            }
        }

        Some((
            preceding_page * XSM_PAGE_LEN,
            succeeding_page * XSM_PAGE_LEN,
        ))
    }

    pub fn get_regs(&self) -> &XSMRegs {
        &self.regs
    }

    // Returns (base_addr, ip, code)
    pub fn get_code(&mut self, max_lines: usize) -> (usize, usize, Vec<String>) {
        let ip: usize = self
            .regs
            .ip
            .parse()
            .expect("IP is not an unsigned integer.");

        // self.xsm.send_line("l");
        // let mut lines = Vec::new();
        // while let Ok(line) = self.xsm.read_line() {
        //     let mut split = line.split('\t');
        //     split.next().unwrap();
        //     lines.push(split.next().unwrap().to_string());
        // }
        // (ip-20, ip, lines)
        let max_addr = max_lines * 2;
        let start;
        let code = if let Mode::User = self.mode {
            let max_range = Self::get_valid_mem_range(ip, &self.page_table)
                .expect("IP not found in page table.");
            let start_ = std::cmp::max(ip - max_addr / 2, max_range.0);
            start = start_ + (start_ % 2);
            let end_ = std::cmp::min(ip + max_addr - max_addr / 2, max_range.1);
            let end = end_ - (end_ % 2);
            Self::read_mem_range_vir(&mut self.xsm, start, end, &self.page_table).unwrap()
        } else {
            let start_ = std::cmp::max(ip - max_addr / 2, 0);
            start = start_ + (start_ % 2);
            // FIXME
            let end_ = std::cmp::min(ip + max_addr - max_addr / 2, 99999);
            let end = end_ - (end_ % 2);
            Self::read_mem_range(&mut self.xsm, start, end)
        };
        let code = code.chunks_exact(2).map(|c| c[0].clone() + &c[1]).collect();
        (start, ip, code)
    }

    /// If not halted returns (self, program output)
    pub fn step(&mut self) {
        self.xsm.send_line("step").expect("Failed to send `step`.");
        if let Some(rexpect::process::wait::WaitStatus::Exited(_, _)) = self.xsm.process.status() {
            self.halted = true;
            let _ = Self::_read_status(&mut self.xsm).map(|(mode, output)| {
                self.mode = mode;
                self.output = output
            });
            return;
        }
        self.load_state().expect("Failed to load state");
    }
}
