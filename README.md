Rust Doom
=========

A little Doom 1 & 2 Renderer written in [Rust](https://github.com/rust-lang/rust). I wanted to learn the language and was inspired by Notch's [Dart Doom renderer](https://github.com/xNotch/Dark), who is using his project to learn Dart better. Mostly it's a fun little project, but I will take PR-s to do with compatibility issues and the like.

The code is mostly based on the endlessly useful [Doom Wiki](http://doom.wikia.com) and the [Unofficial Doom Specs](http://aiforge.net/test/wadview/dmspec16.txt). It is **not** a port of the original Doom C source code into Rust; I'm doing my best to make the code as idiomatic as possible and have not even looked at the original in a long time.

### Build Instructions
I build against nightlies; I pull rustc every few days or so. Give me a shout (submit an issue) if it doesn't build on the most recent nightly and I'll fix it ASAP.

To build use [cargo](http://crates.io):
```
$ cargo build --release
$ targets/release/rs-doom --iwad doom1.wad --level 0
```

Notice you'll need a WAD file (the Doom assets). If you own the game (or are willing to buy it for a few quid/bucks), you'll find it in your game files. If not, you can use probably find the shareware ones floating around on the interwebs. Alternatively you can use the [Freedoom](freedoom.github.io) wads, or, though I would not know of such things, you may be able to obtain them for free in less.... savoury parts of the internet.


## Goals
_(subject to change)_

* **Modern OpenGL >3 renderer.** No immediate mode shenanigans: it's all VBO-s and shaders. Unlike some GL ports, the floors are actually rendered as convex polygons computed from the BSP. The downside is some visual glitches (like [slime trails](http://doom.wikia.com/wiki/Slime_trail)) don't show up, or show up differently.
* **Correct 256 color palette.** Uses the original palette and colormaps to replicate the original lighting effects (mostly you'll notice things get darker in visible steps and they also get greyer as they get darker). Doing 256 color palette lookups in a fragment shader is wonderfully anachronistic.
* **Free flying camera.** Mouse & keyboard control for full 6 degrees of freedom.

## Screenshots
![Zig-zag Screenshot](screenshots/readme1.png)
![Doom 2 Screenshot](screenshots/readme2.png)

## Todo
* [x] BSP -> convex subsector conversion.
* [x] Flats (floors & ceiling) texture mapping.
* [x] Wall texture mapping.
* [x] Wall texture alignment.
* [x] Scrolling walls.
* [ ] Animated flats.
* [ ] Animated walls.
* [ ] Sky rendering.
* [ ] Sprite rendering.
* [ ] Camera-world collisions.
* [ ] Correct player movement (falling etc.)
* [ ] Sprite-player and sprite-sprite collisions.
