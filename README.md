# FOSSO
FOSSO or Free & Open Source Scoreboard Overlay is an indev program that designed to provide a clean & customizable program to chromakey for sports on youtube.
# Installation
Pre-compiled binaries will be under [releases](https://github.com/AllLiver/FOSSO/releases "releases")
If your platform does not have a pre-compiled binary please follow the instructions to [compile](https://github.com/AllLiver/FOSSO?tab=readme-ov-file#compilation "how to compile") repository yourself

# Compilation 
- Download the source code from the latest [release](https://github.com/AllLiver/FOSSO/releases "releases")
- Install [Rust](https://rustup.rs/ "rustup") if you have not
- Run this command in the same directory as the cloned repository
```
cargo build --release
```
The compiled binary will be in /target/release

# Roadmap
FOSSO is an indev project so there is lots likely to change  
Here are some features/updates planned
 - Fix naming in API and frontend
 - Add a sponsor roll in the bottom right (that can be turned on and off)
 - Add savable school configurations with their pictures, names, and colors
 - A countdown clock for halftime or delays
