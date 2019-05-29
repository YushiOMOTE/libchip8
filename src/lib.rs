#![no_std]

use log::trace;

pub trait Hardware: Sized {
    fn rand(&mut self) -> u8;

    fn key(&mut self, key: u8) -> bool;

    fn vram_set(&mut self, x: u8, y: u8, d: bool);
    fn vram_get(&mut self, x: u8, y: u8) -> bool;
    fn vram_setsize(&mut self, size: (u8, u8));
    fn vram_size(&mut self) -> (u8, u8);

    /// Return the current clock value in nanoseconds
    fn clock(&mut self) -> u64;

    /// Play beep sound
    fn beep(&mut self);

    /// Called in every step; return `true` for shutdown
    fn sched(&mut self) -> bool {
        false
    }
}

pub struct Chip8<T> {
    v: [u8; REGS],
    i: u16,
    vf: bool,
    dt: u8,
    st: u8,
    pc: u16,
    sp: u8,
    mem: [u8; MEMS],
    stack: [u16; STACKS],
    time: Option<u64>,
    running: bool,
    hw: T,
}

const REGS: usize = 16;
const MEMS: usize = 4096;
const STACKS: usize = 16;
const DISPS: (u8, u8) = (64, 32);
const ENTRY: u16 = 512;
const ROMBASE: usize = 512;

static CHARBUF: [u8; 80] = [
    0xf0, 0x90, 0x90, 0x90, 0xf0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xf0, 0x10, 0xf0, 0x80, 0xf0, // 2
    0xf0, 0x10, 0xf0, 0x10, 0xf0, // 3
    0x90, 0x90, 0xf0, 0x10, 0x10, // 4
    0xf0, 0x80, 0xf0, 0x10, 0xf0, // 5
    0xf0, 0x80, 0xf0, 0x90, 0xf0, // 6
    0xf0, 0x10, 0x20, 0x40, 0x40, // 7
    0xf0, 0x90, 0xf0, 0x90, 0xf0, // 8
    0xf0, 0x90, 0xf0, 0x10, 0xf0, // 9
    0xf0, 0x90, 0xf0, 0x90, 0x90, // a
    0xe0, 0x90, 0xe0, 0x90, 0xe0, // b
    0xf0, 0x80, 0x80, 0x80, 0xf0, // c
    0xe0, 0x90, 0x90, 0x90, 0xe0, // d
    0xf0, 0x80, 0xf0, 0x80, 0xf0, // e
    0xf0, 0x80, 0xf0, 0x80, 0x80, // f
];

impl<T: Hardware> Chip8<T> {
    pub fn new(hw: T) -> Self {
        Self {
            v: [0; REGS],
            i: 0,
            vf: false,
            dt: 0,
            st: 0,
            pc: 0,
            sp: 0,
            mem: [0; MEMS],
            stack: [0; STACKS],
            time: None,
            running: false,
            hw,
        }
    }

    pub fn run(mut self, rom: &[u8]) {
        self.setup();
        self.load(rom);

        while self.running {
            self.sched();
            self.eval();
            self.next();
        }
    }

    fn setup(&mut self) {
        self.pc = ENTRY;
        self.hw.vram_setsize(DISPS);
        self.mem[..CHARBUF.len()].copy_from_slice(&CHARBUF);
        self.running = true;
    }

    fn shutdown(&mut self) {
        self.running = false;
    }

    fn load(&mut self, rom: &[u8]) {
        self.mem[ROMBASE..ROMBASE + rom.len()].copy_from_slice(&rom);
    }

    fn push(&mut self, item: u16) {
        self.sp = self.sp.wrapping_add(1);
        self.stack[self.sp as usize] = item;
    }

    fn pop(&mut self) -> u16 {
        let item = self.stack[self.sp as usize];
        self.sp = self.sp.wrapping_sub(1);
        item
    }

    fn jump(&mut self, pc: u16) {
        self.pc = pc;
    }

    fn next(&mut self) {
        self.jump(self.pc.wrapping_add(2));
    }

    fn sched(&mut self) {
        if self.hw.sched() {
            self.shutdown();
        }

        if let Some(t) = self.time {
            if self.hw.clock().wrapping_sub(t) > 1000_000_000 / 60 {
                self.tick();
                self.time = Some(self.hw.clock());
            }
        } else {
            self.time = Some(self.hw.clock());
        }
    }

    /// Event which happens in 60 Hz interval
    fn tick(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
            if self.st == 0 {
                self.hw.beep();
            }
        }
    }

    fn waitkey(&mut self) -> u8 {
        while self.running {
            self.sched();

            for i in 0..0xf {
                if self.hw.key(i) {
                    return i;
                }
            }
        }

        return b' ';
    }

    fn eval(&mut self) {
        let h = self.mem[self.pc as usize] as u16;
        let l = self.mem[(self.pc + 1) as usize] as u16;
        let inst = h << 8 | l;

        let nnn = inst & 0xfff;
        let n = (inst & 0xf) as u8;
        let x = ((inst >> 8) & 0xf) as usize;
        let y = ((inst >> 4) & 0xf) as usize;
        let kk = (inst & 0xff) as u8;

        match (
            (inst >> 12) & 0xf,
            (inst >> 8) & 0xf,
            (inst >> 4) & 0xf,
            (inst >> 0) & 0xf,
        ) {
            (0, 0, 0xe, 0) => {
                trace!("CLS");
                let (w, h) = self.hw.vram_size();
                for (x, y) in (0..w).map(|w| (0..h).map(move |h| (w, h))).flatten() {
                    self.hw.vram_set(x, y, false);
                }
            }
            (0, 0, 0xe, 0xe) => {
                trace!("RET");
                let addr = self.pop();
                self.jump(addr);
            }
            (0, _, _, _) => {
                trace!("SYS nnn");
                unimplemented!()
            }
            (1, _, _, _) => {
                trace!("JP nnn");
                self.jump(nnn.wrapping_sub(2));
            }
            (2, _, _, _) => {
                trace!("CALL nnn");
                self.push(self.pc);
                self.jump(nnn.wrapping_sub(2));
            }
            (3, _, _, _) => {
                trace!("SE Vx kk");
                if self.v[x] == kk {
                    self.next();
                }
            }
            (4, _, _, _) => {
                trace!("SNE Vx, kk");
                if self.v[x] != kk {
                    self.next();
                }
            }
            (5, _, _, 0) => {
                trace!("SE Vx, Vy");
                if self.v[x] == self.v[y] {
                    self.next();
                }
            }
            (6, _, _, _) => {
                trace!("LD Vx, kk");
                self.v[x] = kk;
            }
            (7, _, _, _) => {
                trace!("ADD Vx, kk");
                self.v[x] = self.v[x].wrapping_add(kk);
            }
            (8, _, _, 0) => {
                trace!("LD Vx, Vy");
                self.v[x] = self.v[y];
            }
            (8, _, _, 1) => {
                trace!("OR Vx, Vy");
                self.v[x] |= self.v[y];
            }
            (8, _, _, 2) => {
                trace!("AND Vx, Vy");
                self.v[x] &= self.v[y];
            }
            (8, _, _, 3) => {
                trace!("XOR Vx, Vy");
                self.v[x] ^= self.v[y];
            }
            (8, _, _, 4) => {
                trace!("ADD Vx, Vy");
                let (v, c) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = v;
                self.vf = c;
            }
            (8, _, _, 5) => {
                trace!("SUB Vx, Vy");
                let (v, b) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = v;
                self.vf = !b;
            }
            (8, _, _, 6) => {
                trace!("SHR Vx, Vy");
                self.vf = (self.v[x] & 1) == 1;
                self.v[x] = self.v[x].wrapping_shr(1);
            }
            (8, _, _, 7) => {
                trace!("SUBN Vx, Vy");
                let (v, b) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = v;
                self.vf = !b;
            }
            (8, _, _, 0xe) => {
                trace!("SHL Vx, Vy");
                self.vf = (self.v[x] & 0x80) == 0x80;
                self.v[x] = self.v[x].wrapping_shl(1);
            }
            (9, _, _, 0) => {
                trace!("SNE Vx, Vy");
                if self.v[x] != self.v[y] {
                    self.next();
                }
            }
            (0xa, _, _, _) => {
                trace!("LD I, nnn");
                self.i = nnn;
            }
            (0xb, _, _, _) => {
                trace!("JP V0, nnn");
                self.jump(nnn.wrapping_add(self.v[0].into()).wrapping_sub(2));
            }
            (0xc, _, _, _) => {
                trace!("RND Vx, kk");
                self.v[x] = self.hw.rand() & kk;
            }
            (0xd, _, _, _) => {
                trace!("DRW Vx, Vy, n");
                let basex = self.v[x];
                let basey = self.v[y];
                let (w, h) = self.hw.vram_size();

                for y in 0..n {
                    let b = self.mem[(self.i + y as u16) as usize];

                    let vramy = (y + basey) % h;

                    for x in 0..8 {
                        let vramx = (x + basex) % w;

                        let src = b & (1 << (7 - x)) > 0;
                        let dst = self.hw.vram_get(vramx, vramy);

                        self.vf |= src && dst;
                        self.hw.vram_set(vramx, vramy, src ^ dst);
                    }
                }
            }
            (0xe, _, 9, 0xe) => {
                trace!("SKP Vx");
                if self.hw.key(self.v[x]) {
                    self.next();
                }
            }
            (0xe, _, 0xa, 0x1) => {
                trace!("SKNP Vx");
                if !self.hw.key(self.v[x]) {
                    self.next();
                }
            }
            (0xf, _, 0, 7) => {
                trace!("LD Vx, DT");
                self.v[x] = self.dt;
            }
            (0xf, _, 0, 0xa) => {
                trace!("LD Vx, K");
                self.v[x] = self.waitkey();
            }
            (0xf, _, 1, 5) => {
                trace!("LD DT, Vx");
                self.dt = self.v[x];
            }
            (0xf, _, 1, 8) => {
                trace!("LD ST, Vx");
                self.st = self.v[x];
            }
            (0xf, _, 1, 0xe) => {
                trace!("ADD I, Vx");
                self.i = self.i.wrapping_add(self.v[x].into());
            }
            (0xf, _, 2, 9) => {
                trace!("LD F, Vx");
                self.i = (self.v[x] * 5).into();
            }
            (0xf, _, 3, 3) => {
                trace!("LD B, Vx");
                let bcd = self.v[x];
                self.mem[self.i as usize] = bcd / 100;
                self.mem[self.i as usize + 1] = (bcd / 10) % 10;
                self.mem[self.i as usize + 2] = bcd % 10;
            }
            (0xf, _, 5, 5) => {
                trace!("LD [I], Vx");
                for i in 0..self.v.len() {
                    self.mem[self.i as usize + i] = self.v[i];
                }
            }
            (0xf, _, 6, 5) => {
                trace!("LD Vx, [I]");
                for i in 0..self.v.len() {
                    self.v[i] = self.mem[self.i as usize + i];
                }
            }
            _ => panic!("[{:04x}] Invalid op: {:04x}", self.pc, inst), // Bad ops
        }
    }
}
