# devspace - save & retrieve your devlopment workspaces.

devspace helps you to quickly start all the programs you need to dev, like run
`tmux` split the window vertically then horizontally, open programs inside your
panes etc ..

## To-do

Add support for config file and space types:

```ron
Config(
    spacetypes: {
        "default": TmuxVSplit(
            lhs: TMuxCmd("hx"),
            rhs: TMuxHSplit(
                top: None,
                bottom: None,
            )
        ),
        "jump": Cmd("cd", SpaceBase),
    }
)
```
In this example of config, there is a space type, "default" that when launched,
first splits the screen vertically, in the left, the Helix Editor is runned and
in the right there is an horizontal split with the two panes containing no
other split nor a command is run, just the system default's shell.

There is another space type, "jump", it runs just one command in the system's
default shell, "cd" with the Space Base as its argument.
