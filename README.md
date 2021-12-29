# Snake
### By Nikhil Ivaturi

## Install

Make sure [cargo](https://www.rust-lang.org/learn/get-started) is installed on your computer. Clone this repository, cd into it, and run `cargo run` to play.

## Controls

Use the arrow keys to change the direction of the snake.

## Credits

Made using [Bevy](https://bevyengine.org/)

From [this tutorial](https://mbuffett.com/posts/bevy-snake-tutorial/)

## Changes

Changes from tutorial:
* Better system for restarting upon death
    * All components with the `Size` component are queued (happens to be all components that need to be reset) and those are despawned
* Solved opposite direction input bug
    * Created `QueuedInput` component that is updated on input, this component is then read during the movement state
* Food cannot spawn on the snake anymore
    * Created `OccupiedPositions` that would update each movement cycle and keep track of which of these
