# Conway's Game of Life

An implementation of [Conway's Game of Life](https://en.wikipedia.org/wiki/Conway's_Game_of_Life).

[![Build Status](https://travis-ci.com/jonstites/game_of_life.svg?branch=master)](https://travis-ci.com/jonstites/fractious)

## Features

Move around and zoom in and out:

![Move and zoom](https://raw.github.com/jonstites/game_of_life/master/.docs/move_and_zoom.gif?sanitize=true)


Randomize a region:

![Randomize](https://raw.github.com/jonstites/game_of_life/docs/.master/randomize.gif?sanitize=true)

Slow or fast:

![Control speed](https://raw.github.com/jonstites/game_of_life/master/.docs/speed.gif?sanitize=true)

Choose among some hand-picked interesting patterns:

![Choose patterns](https://raw.github.com/jonstites/game_of_life/master/.docs/choose_patterns.gif?sanitize=true)

You can also use some other fun outer-totalistic rulesets:

![Rulesets](https://raw.github.com/jonstites/game_of_life/docs/.master/rulesets.gif?sanitize=true)

## Tech Stack

This implementation uses [Yew](https://github.com/yewstack/yew) framework.

In other words - it is Rust code that is compiled to webassembly, HTML, CSS, and JavaScript.

WebGL is used for rendering.

## Motivation

Because up until now, there isn't a good implementation of Conway's Game of Life.

Kidding, obviously.

I think Rust and webassembly are likely to become way more popular, and that exposure to them generalizes well towards other technologies, and just intrinsic motivation and fun.

## Design

I set out with the following wishlist:

1. infinite or apparently-infinite universe
2. reasonable performance on large, chaotic universes
3. support for multiple rulesets
4. interactivity

Off-the-bat, requirement 1 rules out the easiest and most-straightforward approach: a fixed-size universe of two matrices or lists.

Requirements 2 and 4 rule out the well-known HashLife algorithm. HashLife is incredible, but not well-suited for somebody sitting and watching the results.

I considered using either a tree or a hash table.

Specifically, I was interested in building a quad-tree and maybe even doing some caching of children for additional speedups.

Any tree also has the obvious advantage of avoiding hashing, and therefore performance advantages.

A hash table is dead simple.

After running tests that showed that FNV hash was reasonably fast and unlikely to be a bottleneck, I went with the hash table. 

It has following optimizations:

1. FNV hashing of the integer coordinates
2. cells are calculated in 2x2 blocks from a 4x4 block using bit operations and a lookup table
3. static 4x8 blocks of cells whose 3 neighbors are also static are skipped
4. stagger-step between generations to reduce overhead and number of neighbor blocks that can trigger a 4x8 block to be calculated

Inspiration comes from these sources and source code:

[Golly](https://sourceforge.net/p/golly/code/ci/master/tree/gollybase/qlifealgo.h)

[Alan Hensel](http://www.ibiblio.org/lifepatterns/lifeapplet.html)

[lifelib](https://gitlab.com/apgoucher/lifelib)

[conwaylife](https://www.conwaylife.com/forums/viewtopic.php?f=7&t=3237)

[Graphics Programmer's Black Book](http://www.jagregory.com/abrash-black-book/#chapter-18-its-a-plain-wonderful-life)

