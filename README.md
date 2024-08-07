# FROGGI
[![Rust](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml/badge.svg)](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml)
![GitHub License](https://img.shields.io/github/license/allliver/froggi)  
**F**lexible **R**eal-time **O**verlay for **G**ame **G**raphics and **I**nformation  
Is a self-hosted portable scoreboard solution that aims to provide a free and intuitive sports broadcasting overlay.

# Features
- ✨ Optical character recognition using this [froggi-ocr](https://github.com/AllLiver/froggi-ocr) and [scoresight-ocr](https://github.com/locaal-ai/scoresight), using a video source, the program can be automatic
- ✨ An optional sponsor logo slideshow on overlay
- ✨ Match presets
- ✨ Ui optimized for iPad like devices

# Installation
Froggi is on [Docker Hub](https://hub.docker.com/repository/docker/allliver/froggi/general)  
Alternatively, docker image archives can be found under [releases](https://github.com/AllLiver/Froggi/releases)  
  
If you would like to run Froggi as a standalone executable, then follow the guide on how to [compile](https://github.com/AllLiver/Froggi/edit/dev/README.md#compilation)

# Usage
Once you start Froggi it will prompt you to create a server username and password  
Setting up match presets and sponsors is done through the "Sponsors & Teaminfo" tab in the burger menu  
Other settings can be found in the "Settings" tab in the burger menu  
Finally, there are some niche settings in "settings.json" located in the directory of Froggi's binary, these settings will only take effect on a restart of Froggi  

# Compilation 
- Clone the main branch of Froggi by running
```
git clone https://github.com/allliver/froggi.git
```
- Install the [Rust Toolchain](https://rustup.rs/ "rustup") if you have not
- If you are compiling for Linux, ensure essential C build tools are installed
- Then finally compile Froggi by running
```
cargo build --release
```
The compiled binary will be located in /target/release

# Roadmap
Froggi is an indev project so change is very likley.
Here are some features/updates planned in no particular order
 - Jersey colors will reflect the color on the overlay
 - Options for more sports
 - Web acsessible logs

# Tech Stack
 - Rust with Axum for the backend
 - HTML, CSS, JavaScript, and the HTMX library for the frontend

# Contribute
If you are fammiliar with our tech stack, feel free to submit a pull request!
