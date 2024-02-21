# FROGGI
[![Rust](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml/badge.svg)](https://github.com/AllLiver/Froggi/actions/workflows/rust.yml)
![GitHub License](https://img.shields.io/github/license/allliver/froggi)  
**F**lexible **R**eal-time **O**verlay for **G**ame **G**raphics and **I**nformation  
Is a self-hosted scoreboard overlay software that aims to provide an easy-to-use overlay for sports broadcasting!

# Usage
When you run the binary for the first time it should generate all the files and folders it needs  
Here is what each of those files/folders do.
 - sponsors (folder): any png file you put in here will be cycled every 5 seconds if you press the show sponsors button on the dashboard or countdown page, note it will only load these on app startup
 - teams (folder): this folder contains the images, names, and jersey color of all team presets you set
 - login (folder): this folder is not for manual editing and contains login information
 - config.cfg (file): this file is where you can set the address the server listens on and the background color of the overlay page in RGB format  

When logging into the web interface for the first time you will be prompted to create a login for the web interface.  
After creating a login simply sign in, upload team presets, and start streaming!

# Installation
Pre-compiled binaries will be under [releases](https://github.com/AllLiver/FOSSO/releases "releases")
If your platform does not have a pre-compiled binary please follow the instructions to [compile](https://github.com/AllLiver/FOSSO?tab=readme-ov-file#compilation "how to compile") repository yourself

# Compilation 
- Download the source code from the latest [release](https://github.com/AllLiver/FOSSO/releases "releases") (usually main branch is not stable)
- Install [Rust](https://rustup.rs/ "rustup") if you have not
- Make sure you have basic C build tools (Windows and MacOS usually have them pre-installed)
- Run this command in the same directory as the cloned repository
```
cargo build --release
```
The compiled binary will be in /target/release

# Roadmap
Froggi is an indev project so change is very likley.
Here are some features/updates planned
 - More consistent naming in API and frontend code
 - Logs you can access in the web app
 - Ui popups for team touchdowns, & scoring
 - Acsessibility improvements, such as image alts

# Tech Stack
 - Rust with Axum in the backend
 - HTML, CSS, JavaScript, and the HTMX library for the frontend

# Contribute
If you are fammiliar with our tech stack, feel free to submit a pull request!
