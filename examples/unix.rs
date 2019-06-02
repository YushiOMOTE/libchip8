use log::*;
use minifb::{Key, Scale, Window, WindowOptions};
use rand::{rngs::ThreadRng, Rng};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// Log file
    #[structopt(short = "l", long = "log")]
    log: Option<String>,
    /// Log level
    #[structopt(short = "d", long = "loglevel", default_value = "info")]
    loglevel: String,
    /// Clock speed in Hz
    #[structopt(short = "c", long = "clock", default_value = "1000")]
    hz: u64,
}

struct Hardware {
    win: Option<Window>,
    rng: ThreadRng,
    inst: std::time::Instant,
    vramsz: (usize, usize),
    vram: Vec<bool>,
    opt: Opt,
}

impl Hardware {
    fn new(opt: Opt) -> Self {
        Self {
            win: None,
            rng: rand::thread_rng(),
            inst: std::time::Instant::now(),
            vramsz: (0, 0),
            vram: vec![],
            opt,
        }
    }
}

impl libchip8::Hardware for Hardware {
    fn rand(&mut self) -> u8 {
        self.rng.gen()
    }

    fn key(&mut self, key: u8) -> bool {
        let k = match key {
            0 => Key::X,
            1 => Key::Key1,
            2 => Key::Key2,
            3 => Key::Key3,
            4 => Key::Q,
            5 => Key::W,
            6 => Key::E,
            7 => Key::A,
            8 => Key::S,
            9 => Key::D,
            0xa => Key::Z,
            0xb => Key::C,
            0xc => Key::Key4,
            0xd => Key::E,
            0xe => Key::D,
            0xf => Key::C,
            _ => return false,
        };

        match &self.win {
            Some(win) => win.is_key_down(k),
            None => false,
        }
    }

    fn vram_set(&mut self, x: usize, y: usize, d: bool) {
        trace!("Set pixel ({},{})", x, y);
        self.vram[(y * self.vramsz.0) + x] = d;
    }

    fn vram_get(&mut self, x: usize, y: usize) -> bool {
        self.vram[(y * self.vramsz.0) + x]
    }

    fn vram_setsize(&mut self, size: (usize, usize)) {
        self.vramsz = size;
        self.vram = vec![false; size.0 * size.1];

        let win = match Window::new(
            "Chip8",
            64,
            32,
            WindowOptions {
                resize: true,
                scale: Scale::X4,
                ..WindowOptions::default()
            },
        ) {
            Ok(win) => win,
            Err(err) => {
                panic!("Unable to create window {}", err);
            }
        };

        self.win = Some(win);
    }

    fn vram_size(&mut self) -> (usize, usize) {
        self.vramsz
    }

    fn clock(&mut self) -> u64 {
        let d = self.inst.elapsed();
        d.as_secs()
            .wrapping_mul(1000_000_000)
            .wrapping_add(d.subsec_nanos().into())
    }

    fn beep(&mut self) {}

    fn sched(&mut self) -> bool {
        std::thread::sleep(std::time::Duration::from_micros(1000_000 / self.opt.hz));

        if let Some(win) = &mut self.win {
            if !win.is_open() || win.is_key_down(Key::Escape) {
                return true;
            }

            let vram: Vec<u32> = self
                .vram
                .clone()
                .into_iter()
                .map(|b| if b { 0xffffff } else { 0 })
                .collect();
            win.update_with_buffer(&vram).unwrap();
        }

        false
    }
}

fn main() {
    let opt = Opt::from_args();

    if let Some(log) = &opt.log {
        use log4rs::append::file::FileAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(log)
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(opt.loglevel.parse().expect("Invalid log level")),
            )
            .unwrap();

        log4rs::init_config(config).unwrap();
    }

    let chip8 = libchip8::Chip8::new(Hardware::new(opt));
    chip8.run(include_bytes!("roms/invaders.ch8"));
}
