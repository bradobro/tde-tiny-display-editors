# Re-Experiencing Visual Display Editor (VDE)

**Origins**: I originally forked from [mecparts/zde](https://github.com/mecparts/zde) to retain history of the initial CP/M assembly, I've disconnected the fork because this repo has very different intentions and should never be merged. **Big Thanks!** to Eric Meyer for creating and sharing VDE and to mecparts for his reconstructive work.)

# Ah VDE!

Beware: software sentimentality ahead.

It was 1987, sophomore year in college, and Turbo Pascal hit the magazine pages and the software shelf of our computer center. Suddenly there was something worth stepping away from the Vax terminal and Apple II to check out some of the mysterious IBM PC's over on the other side of the center.

Besides finding a whole new playground for building software, I discovered a new experience of full-screen editing with WordStar-style keyboard maps. I fell in love with a text editor for the first time.

Shortly after graduation, I bought my first IBM PC. It was about that time Eric Meyer's freeware, *Visual Display Editor* (VDE) showed up and I suddenly had that joy for all my documents. Before long, it edited my personal knowledgebase and writing.

# A Portable and Unicode-Aware Port

Since then I've moved through Windows and now work mostly in Posix-based systems (including my MacBook). My affections have passed through several other editors, and my fingers know Vim keystrokes better than the WordStar mapping. But I still remember the elegant fun of VDE--so much power crammed into such a small executable!

I'm not into DOSBox or CP/M emulators, but having found `mecparts`' [reconstruction of ZDE-style assembly code from VDE assembley](https://github.com/mecparts/zde), was amazed at the efficiency. It occured to me, "What if we had to create VDE to work on modern terminals using a higher-level, portable language? How would it feel? What tradeoffs would I make (like making it 8-bit or Unicode safe)?

Here's the original listing from `mecparts`' repo:

```
...
-rw-r--r--@ 1 bradobro  staff    17K Jul 24 07:48 zde17.com
```

17K executable! For my first take, I choise Rust, knowing that the stdlib inclusion would cost a few 100K and choosing to use one dependency, `crossterm` to help get a working version off the ground. The result: 680K for a release build, 1.7M for the debug build, and it still doesn't have all the features (not even a visible cursor yet)! Smaller than Helix (and less capable), but by no means minimal.

As you can guess, this repo leans heavily on LLM's for the porting. If you're an editor geek, the planning under `doc/decisions/` and `doc/iterations` is instructive about the choices made so far.

I'm not sure what else I'll do with, if anything, but I'm putting it out there on GitHub for any who might be interested.

