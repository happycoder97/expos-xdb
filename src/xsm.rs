use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command};
use std::thread::sleep;
use std::time::Duration;

const XSM_PAGE_LEN: usize = 512;

pub struct XSM {
    xsm: Child,
    stdin: ChildStdin,
    stdout: crossbeam_channel::Receiver<String>,
    mode: Mode,
    regs: XSMRegs,
    page_table: Vec<XSMPageTableEntry>,
    errors: Vec<String>,
    output: Vec<String>,
    is_next_halt: bool,
    is_exception_edge: bool,
    halted: bool,
    status: String,
    last_code: (usize, usize, Vec<String>),
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
    pub phy: String,
    pub aux: String,
}

#[derive(Debug)]
pub enum Mode {
    Kernel,
    User,
}

#[derive(Debug)]
pub struct XSMRegs {
    pub r: [String; 20],
    pub p: [String; 4],
    pub bp: String,
    pub sp: String,
    pub ip: String,
    pub ptbr: String,
    pub ptlr: String,
    pub eip: String,
    pub ec: String,
    pub epn: String,
    pub ema: String,
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

impl XSM {
    pub fn spawn_new(command: &str) -> Result<XSM, ()> {
        let mut stdbuf_args = vec!["--output=0"];
        stdbuf_args.extend(command.split_whitespace());

        let mut xsm_process = Command::new("stdbuf")
            .args(&stdbuf_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| {
                println!("Error: Failed to launch command.");
            })?;

        let stdout = xsm_process.stdout.take().expect("Failed to get stdout");
        let stdin = xsm_process.stdin.take().expect("Failed to get stdin");
        let (stdout_tx, stdout_rx) = crossbeam_channel::bounded(100);
        std::thread::spawn(move || {
            let mut buf_reader = BufReader::new(stdout);
            let mut buf = String::new();
            loop {
                buf_reader.read_line(&mut buf);
                stdout_tx.send(buf.clone());
                buf.clear();
            }
        });

        let mut xsm = XSM {
            xsm: xsm_process,
            stdin,
            stdout: stdout_rx,
            mode: Mode::Kernel,
            regs: XSMRegs::default(),
            page_table: Vec::new(),
            errors: Vec::new(),
            output: Vec::new(),
            is_next_halt: false,
            is_exception_edge: true,
            halted: false,
            status: String::new(),
            last_code: (0, 0, Vec::new()),
        };

        sleep(Duration::from_millis(200));
        if xsm.xsm.try_wait().transpose().is_some() {
            xsm.halted = true;
            println!("Error: xsm exited without entering debug mode.");
            println!("Please check your command line");
            Err(())
        } else {
            xsm.load_state();
            Ok(xsm)
        }
    }

    /// If not halted returns (self, program output)
    pub fn step(&mut self, n: usize) {
        writeln!(self.stdin, "step {}", n).expect("Failed to send command to xsm");
        if let Ok(Some(retcode)) = self.xsm.try_wait() {
            eprintln!("Halted {}", retcode);
            self.halted = true;
            self._read_status();
            return;
        }
        self.load_state();
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn is_next_halt(&self) -> bool {
        self.is_next_halt
    }

    pub fn is_exception_edge(&self) -> bool {
        self.is_exception_edge
    }

    // Returns (base_addr, ip, code)
    pub fn get_code(&mut self, max_lines: usize) -> (usize, usize, Vec<String>) {
        let ip: usize = match self.regs.ip.parse() {
            Ok(ip) => ip,
            Err(_) => {
                self.errors.push("IP is not an unsigned number".to_owned());
                return self.last_code.clone();
            }
        };

        let max_addr = max_lines * 2;
        let start;
        let code = if let Mode::User = self.mode {
            let max_range = match Self::get_valid_mem_range(ip, &self.page_table) {
                Ok(r) => r,
                Err((ip, page)) => {
                    self.errors.push(format!(
                        "IP: {}, Page: {} not found in page table",
                        ip, page
                    ));
                    return self.last_code.clone();
                }
            };
            let start_ =
                std::cmp::max(ip as isize - max_addr as isize / 2, max_range.0 as isize) as usize;
            start = start_ + (start_ % 2);
            let end_ = std::cmp::min(ip + max_addr - max_addr / 2, max_range.1);
            let end = end_ - (end_ % 2);
            self.read_mem_range_vir(start, end)
        } else {
            let start_ = std::cmp::max(ip as isize - max_addr as isize / 2, 0) as usize;
            start = start_ + (start_ % 2);
            // FIXME
            let end_ = std::cmp::min(ip + max_addr - max_addr / 2, 99999);
            let end = end_ - (end_ % 2);
            self.read_mem_range(start, end)
        };
        let code = code.chunks_exact(2).map(|c| c[0].clone() + &c[1]).collect();

        (start, ip, code)
    }

    pub fn get_regs(&self) -> &XSMRegs {
        &self.regs
    }

    pub fn get_page_table(&self) -> &Vec<XSMPageTableEntry> {
        &self.page_table
    }

    pub fn get_errors(&self) -> &Vec<String> {
        &self.errors
    }

    pub fn get_output(&self) -> &Vec<String> {
        &self.output
    }

    pub fn get_status(&self) -> &str {
        &self.status
    }

    fn get_stdout(&mut self, lines: usize) -> Vec<String> {
        let mut vec = Vec::with_capacity(lines);
        if lines == 0 {
            while let Ok(s) = self.stdout.recv_timeout(Duration::from_millis(10)) {
                vec.push(s);
            }
        } else {
            for _ in 0..lines {
                vec.push(
                    self.stdout
                        .recv_timeout(Duration::from_millis(1000))
                        .map_err(|e| {
                            eprintln!("Read so far");
                            dbg!(&vec);
                        })
                        .unwrap(),
                );
            }
        }
        vec
    }

    /// Must be called right after entering debug mode
    /// or right after sending step command
    /// Returns: Mode, Program output if there was an out instruction
    fn load_state(&mut self) {
        self._read_status();
        if self.is_halted() {
            return;
        }
        self._read_regs();
        self._read_page_table();
    }

    /// ------------ Called by load state --------------- ///
    fn _read_status(&mut self) {
        let mut lines = self.get_stdout(0);
        if lines.len() < 3 {
            dbg!(lines);
            return;
        }
        if lines[0]
            .trim_start_matches("debug> ")
            .starts_with("Machine is halting.")
        {
            self.halted = true;
            return;
        }
        lines[0] = lines[0].trim_start_matches("debug> ").to_owned();

        self.status.clear();
        for line in lines.iter().skip(lines.len() - 3) {
            self.status.push_str(line);
        }

        for line in lines.iter().take(lines.len() - 3) {
            self.output.push(line.clone());
        }

        let next_instr_line = lines.last().unwrap();
        self.is_next_halt = next_instr_line
            .split(": ")
            .last()
            .unwrap()
            .starts_with("HALT");

        let next_ip_str: &str = next_instr_line
            .split("IP = ")
            .last()
            .unwrap()
            .split(",")
            .next()
            .unwrap();
        let next_ip: u32 = next_ip_str.parse().unwrap();
        self.is_exception_edge = next_ip == 1024;

        let mode_line = &lines[lines.len() - 2];
        let mode_char = mode_line.chars().nth(6).unwrap();
        self.mode = match mode_char {
            'K' => Mode::Kernel,
            'U' => Mode::User,
            _ => panic!("Unexpected mode: '{}'\nLines read: {:#?}", mode_char, lines),
        };
    }

    fn _read_regs(&mut self) {
        writeln!(self.stdin, "reg").expect("Failed to send command to xsm");
        let lines = self.get_stdout(7);

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
        for line in lines {
            for word in line.split('\t') {
                if word == "\n" {
                    continue;
                }
                let val = word.split(": ").nth(1).unwrap();
                let reg = ref_table(i, &mut self.regs);
                reg.clear();
                reg.push_str(val);
                i += 1;
            }
        }
    }

    fn _read_page_table(&mut self) {
        self.page_table.clear();
        let ptbr: usize = if let Ok(ptbr) = self.regs.ptbr.parse() {
            ptbr
        } else {
            self.errors
                .push(format!("PTBR: '{}' is invalid", self.regs.ptbr));
            return;
        };
        let ptlr: usize = if let Ok(ptlr) = self.regs.ptlr.parse() {
            ptlr
        } else {
            self.errors
                .push(format!("PTLR: '{}' is invalid", self.regs.ptlr));
            return;
        };
        let page_table_str = self.read_mem_range(ptbr, ptbr + ptlr * 2);
        for entry_mem in page_table_str.chunks_exact(2) {
            let entry = XSMPageTableEntry {
                phy: entry_mem[0].clone(),
                aux: entry_mem[1].clone(),
            };
            self.page_table.push(entry);
        }
    }
    /// ------------ End of called by load state --------------- ///

    fn _pageify(start_addr: usize, end_addr: usize) -> (usize, usize, usize, usize) {
        let start_page = start_addr / XSM_PAGE_LEN;
        let start_page_skip = start_addr - start_page * XSM_PAGE_LEN;
        let end_page = end_addr / XSM_PAGE_LEN;
        let end_page_take = end_addr - end_page * XSM_PAGE_LEN;
        (start_page, end_page, start_page_skip, end_page_take)
    }

    fn _page_vir_to_phy(&self, vir_page: usize) -> Result<usize, XSMInternalError> {
        if vir_page > self.page_table.len() {
            return Err(XSMInternalError::VirtualMemoryOutOfBounds {
                addr: vir_page * XSM_PAGE_LEN,
            });
        }
        let page_table_entry = &self.page_table[vir_page];
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

    fn read_mem_page(&mut self, page: usize) -> Vec<String> {
        writeln!(self.stdin, "mem {}", page).expect("Failed to send command to xsm");
        let _buf = self.get_stdout(1);
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

    pub fn read_mem_range(&mut self, start_addr: usize, end_addr: usize) -> Vec<String> {
        let mut data = Vec::new();
        let (start_page, end_page, start_page_skip, end_page_take) =
            Self::_pageify(start_addr, end_addr);
        if start_page == end_page {
            data.extend(
                self.read_mem_page(start_page)
                    .into_iter()
                    .take(end_page_take)
                    .skip(start_page_skip),
            );
        } else {
            data.extend(
                self.read_mem_page(start_page)
                    .into_iter()
                    .skip(start_page_skip),
            );
            for i in start_page + 1..end_page {
                data.extend(self.read_mem_page(i).into_iter());
            }
            data.extend(self.read_mem_page(end_page).into_iter().take(end_page_take));
        }
        data
    }

    pub fn read_mem_range_vir(&mut self, start_addr: usize, end_addr: usize) -> Vec<String> {
        let mut data = Vec::new();
        let (start_page_vir, end_page_vir, start_page_skip, end_page_take) =
            Self::_pageify(start_addr, end_addr);
        let start_page_phy = try_or!(self._page_vir_to_phy(start_page_vir), Vec::new());
        if start_page_vir == end_page_vir {
            data.extend(
                self.read_mem_page(start_page_phy)
                    .into_iter()
                    .take(end_page_take)
                    .skip(start_page_skip),
            );
        } else {
            data.extend(
                self.read_mem_page(start_page_phy)
                    .into_iter()
                    .skip(start_page_skip),
            );
            for page_vir in start_page_vir + 1..end_page_vir {
                let page_phy = try_or!(self._page_vir_to_phy(page_vir), Vec::new());
                data.extend(self.read_mem_page(page_phy).into_iter());
            }
            let end_page_phy = try_or!(self._page_vir_to_phy(end_page_vir), Vec::new());
            data.extend(
                self.read_mem_page(end_page_phy)
                    .into_iter()
                    .take(end_page_take),
            );
        }
        data
    }

    /// Return Ok(start_range, end_range) or Err(IP, Page)
    fn get_valid_mem_range(
        include_addr: usize,
        page_table: &Vec<XSMPageTableEntry>,
    ) -> Result<(usize, usize), (usize, usize)> {
        let page = include_addr / XSM_PAGE_LEN;
        let pt_entry: &XSMPageTableEntry = match page_table.get(page) {
            Some(e) => e,
            None => {
                return Err((include_addr, page));
            }
        };
        let _phy: usize = match pt_entry.phy.parse().ok() {
            Some(e) => e,
            None => {
                return Err((include_addr, page));
            }
        };

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

        Ok((
            preceding_page * XSM_PAGE_LEN,
            succeeding_page * XSM_PAGE_LEN,
        ))
    }
}
