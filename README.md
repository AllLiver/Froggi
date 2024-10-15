# FROGGI
[![Rust](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml/badge.svg)](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml)
![GitHub License](https://img.shields.io/github/license/allliver/froggi)  
**F**lexible **R**eal-time **O**verlay for **G**ame **G**raphics and **I**nformation  
Is a self-hosted portable scoreboard solution that aims to provide an intuitive and simple sports broadcasting overlay.

# Features
 - ✨ Optical character recognition (OCR) using [froggi-ocr](https://github.com/AllLiver/froggi-ocr) and [scoresight-ocr](https://github.com/locaal-ai/scoresight)
 - ✨ Authentication through an API key in an HTTP header (allowing for authentication for Bitfocus Companion, or other use cases)
 - ✨ An optional sponsor logo slideshow
 - ✨ Game presets
 - ✨ Ui optimized for iPad like devices

# Installation
Froggi is on [Docker Hub](https://hub.docker.com/repository/docker/allliver/froggi/general)  
Alternatively, Docker image archives can be found under [releases](https://github.com/AllLiver/Froggi/releases)  
  
If you would like to run Froggi as a standalone executable, then follow the guide on how to [compile](https://github.com/AllLiver/Froggi/README.md#compilation)

# Usage
If running as a standalone executable, make sure to start the "froggi" binary, not the "froggi-worker" binary.  
If running under Docker, simply start the container.  

# Setup
After running for the first time, navigate to the web interface (default port is 3000).  
You will then be prompted to create the login for Froggi's interface, and after creating it will be sent to the dashboard.  
You can then set game presets and upload sponsor images in the "Sponsors & Teaminfo" page under the burger menu.  
There is additional configuration under the "Settings" page under the burger menu also. For specialized configuration options (sponsor wait time, countdown opacity, etc.) modify the config.json automatically generated by Froggi upon first run.  
Modifications to config.json are automatically applied upon restarting Froggi, the easiest way to restart/stop Froggi are the program controls at the bottom of the "Settings" page in the burger menu.  

# Roadmap
Froggi is an indev project so changes are very likley.
Here are some features/updates planned in no particular order
 - 🗺️ Support for more sports
 - 🗺️ Pop-up animations
 - 🗺️ Devices connected counter

# Platform support
## Windows
Froggi has full Windows support (with one exception stated on the next line), and binaries under [releases](https://github.com/AllLiver/Froggi/releases).  
However due to the way Windows signals work, you should never stop Froggi by simply doing Ctrl+c in the terminal and instead stop Froggi through the program controls at the bottom of settings in the web interface.  
It is heavily suggested to run Froggi under WSL or Docker due to the majority of backend development happening on Linux.  

## MacOS
Froggi has full MacOS support, however due to the difficulty in cross-compiling for MacOS precompiled binaries are not offered. Detailed instructions on how to [compile from source](https://github.com/AllLiver/Froggi/README.md#compilation) are found below.  
If you would not like to compile the binaries from source please use Docker.

## Linux
Froggi has full Linux support, and binaries under [releases](https://github.com/AllLiver/Froggi/releases).

## Docker
Froggi has full Docker support, and an image on [Docker Hub](https://hub.docker.com/repository/docker/allliver/froggi/general).  
It is the best way to run Froggi if you are familiar with Docker.  

# Updating
Froggi is able to update itself, and if an update is availible you will be able to update it through the Settings page.  
Updates are compiled from source, and in order to update Froggi needs all [build dependencies](https://github.com/AllLiver/Froggi/dependencies) installed.  
The Docker image comes with everything needed to compile updates from source.

# Compilation 
## Dependencies
 - [Rust Toolchain](https://rustup.rs)
 - Essential C build tools (MacOS & Linux platforms only, Rustup on Windows should prompt you to install them)
     - Developer Tools on MacOS
     - GCC on Linux platforms (usually installed under base-devel, build-essential, or any package simmilar to that)
 - OpenSSL libraries and headers (Linux only, see https://docs.rs/openssl/latest/openssl/ for more info)
 - Git (also included with Developer Tools on MacOS)

Once all the build dependancies are installed, clone Froggi's git repository with:
```
git clone https://github.com/AllLiver/Froggi.git
```
Then cd into the repository with:
```
cd Froggi
```
Finally compile Froggi with:
```
cargo build --release
```
The compiled binaries will be under target/release/froggi(.exe if on Windows) and target/release/froggi-worker(.exe if on Windows).  
When running the binaries, only run froggi, not froggi-worker. The two binaries also must be in the same directory as each other.  
Or run them in place with cargo with:
```
cargo run --release --bin froggi
```

# Tech Stack
 - Rust and Axum for the backend
 - HTML, CSS, JavaScript, and the HTMX library for the frontend

# Licence
Froggi is licenced under the permissive [MIT licence](https://mit-license.org/).
