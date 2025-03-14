# devspace - save & retrieve your devlopment workspaces.

> [!WARNING]
> THIS PROJECT IS WORK IN PROGRESS! USE THIS PROGRAM AT YOUR RISK, FOR NOW ITS
> BEHAVIOR ISN'T REALLY LOGIC.

devspace helps you to quickly start all the programs you need to dev, like run
`tmux` split the window vertically then horizontally, open programs inside your
panes etc ..

## Installation

_Devspace isn't yet published to crates.io or any other package, so you need to
manually install it._

### Prerequisites

Cargo and Rust `1.85.0`.

### Guide

1. Clone the repo, `$ git clone git@github.com:thi8v/devspace.git`
2. Move into it, `$ cd devspace`
3. Install it locally through cargo, `$ cargo install --path .`
4. Use it `$ devspace ...` if it doesn't work make sure the directory where
   cargo puts the Rust binaries is in your `PATH` environment variable.

## Devspace directory

Devspace uses the filesystem to store some stuff and has one directory.
This directory contains the following:
- `config.ron`: the configuration of devspace, the Trees and the default Tree
- `db.ron`: the Space with their working directory and their Tree.

The directory is evaluated based on the following priorities:
1. the `--dir <path>` argument.
2. the `DEVSPACE_DIR` global variable.
3. and fallbacks to the default `$HOME/.devspace/`.

## Definitions

**Space**: its a directory and more data, stored in the database, usualy the
directory of a project and its tree.

**Tree**: the programs and its hierarchy that are run when launching a Space.
It's a Tree composed of things like `TmuxVSplit(..)`, `TmuxHSplit(..)`,
`Cmd(..)`, etc.. see more below.

## Usage

To create a new Space for the current directory,
```sh
$ devspace init
```

Or to create a new Space for a specific directory

```sh
$ devspace init /path/to/my/veryspecific/directory
```

You can print the list of spaces stored,
```sh
$ devspace list-spaces
```

To remove a Space, use the `remove` subcommand with the Space name,
```sh
$ devspace remove SPACE_NAME_HERE
```

And the most useful command from all of them, `go`!
```
$ devspace go SPACE_NAME_HERE
```
it will launch your Space with its configured Tree. 

.. or just type
```
$ devspace --help
```
to get some help and discover sub commands, arguments flags etc..

## Trees

> [!WARNING]
> PLEASE NOTE THAT THIS PROJECT IS WORK IN PROGRESS BUT THIS PART IS EVEN MORE
> IN PROGRESS, EXPECT BUGS AND NON-LOGIC BUG / BEHAVIOR OF THIS PROGRAM.

### Cmd

This Tree will run the specified command in the shell. The command has
placeholders, in the Command String you can put `{ .. }` and between the
brackets you can put variable names that will be replaced when launched.

Support variable names in placeholders:
`Space.wdir` : replaced with the Space's working directory path when launched
 _That's it actually lmao_

```ron
Cmd(COMMAND_STRING)
```

### TmuxVSplit

This tree will make a Vertical split in the Tmux session, with one the left
(lhs) its own tree, and same on the right (rhs).

```ron
TmuxVSplit(
 rhs: ANOTHER_TREE, // optional
 lhs: ANOTHER_TREE, // optional
)
```

### TmuxHSplit

This tree will make an Horizontal split in the Tmux session, with one the left
(lhs) its own tree, and same on the right (rhs).

```ron
TmuxVSplit(
 rhs: ANOTHER_TREE, // optional
 lhs: ANOTHER_TREE, // optional
)
```


### Example

Here is a (working lmao) example of a tree,

```ron
TmuxVSplit(
    lhs: Cmd("clear && hx "),
    rhs: TmuxHSplit(),
)
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Feel free to contribute. For the moment there is no documentation but it will come.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
