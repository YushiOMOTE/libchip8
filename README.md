libchip8
=========================

An OS-independent chip8 interpreter library written in Rust (`no_std`).

Once you implement OS-specific part, i.e. `Hardware` trait, you will get a complete chip8 interpreter for your environment.

```rust
struct Hardware;

// 1. Implement `libchip8::Hardware`
impl libchip8::Hardware for Hardware {
   ...
}

// 2. Run `Chip8` giving a rom binary.
let chip8 = libchip8::Chip8::new(Hardware);
chip8.run(include_bytes!("roms/invaders.ch8"));
```

### Example

```
$ cargo run --example unix
```

This example is to run the chip8 interpreter on unix. It uses `minifb` for its graphics.
