use rand::{rngs::ThreadRng, Rng};
use rustty::{
    ui::{Alignable, HorizontalAlign, VerticalAlign, Widget},
    CellAccessor, Event, Terminal,
};

const BLOCK: char = '\u{2588}';

struct Hardware {
    term: Terminal,
    canvas: Widget,
    rng: ThreadRng,
    inst: std::time::Instant,
    vramsz: (u8, u8),
    vram: Vec<bool>,
}

impl Hardware {
    fn new() -> Self {
        let term = Terminal::new().unwrap();
        let mut canvas = Widget::new(256, 256);

        canvas.align(&term, HorizontalAlign::Left, VerticalAlign::Top, 0);

        Self {
            term,
            canvas,
            rng: rand::thread_rng(),
            inst: std::time::Instant::now(),
            vramsz: (0, 0),
            vram: vec![],
        }
    }
}

impl libchip8::Hardware for Hardware {
    fn rand(&mut self) -> u8 {
        self.rng.gen()
    }

    fn key(&mut self, key: u8) -> bool {
        match self.term.get_event(std::time::Duration::from_secs(0)) {
            Ok(Some(Event::Key(ch))) => {
                if ch == key as char {
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    fn vram_set(&mut self, x: u8, y: u8, d: bool) {
        let cell = self.canvas.get_mut(x as usize, y as usize).unwrap();

        if d {
            cell.set_ch(BLOCK);
        } else {
            cell.set_ch(' ');
        }

        // let x = x as usize;
        // let y = y as usize;
        // let w = self.vramsz.0 as usize;
        // self.vram[w * y + x] = d;
    }

    fn vram_get(&mut self, x: u8, y: u8) -> bool {
        let cell = self.canvas.get(x as usize, y as usize).unwrap();
        cell.ch() != ' '

        // let x = x as usize;
        // let y = y as usize;
        // let w = self.vramsz.0 as usize;
        // self.vram[w * y + x]
    }

    fn vram_setsize(&mut self, size: (u8, u8)) {
        let (w, h) = (size.0 as usize, size.1 as usize);
        self.vramsz = size;
        self.vram = vec![false; w * h];
    }

    fn vram_size(&mut self) -> (u8, u8) {
        self.vramsz
    }

    fn clock(&mut self) -> u64 {
        let d = self.inst.elapsed();
        d.as_secs()
            .wrapping_mul(1000000000)
            .wrapping_add(d.subsec_nanos().into())
    }

    fn beep(&mut self) {}

    fn sched(&mut self) -> bool {
        if self.key(b'q') {
            true
        } else {
            self.canvas.draw_into(&mut self.term);
            self.term.swap_buffers().unwrap();
            false
        }
    }
}

fn main() {
    env_logger::init();

    let chip8 = libchip8::Chip8::new(Hardware::new());
    chip8.run(include_bytes!("maze.ch8"))
}
