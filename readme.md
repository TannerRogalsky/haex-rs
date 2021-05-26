# HAEX

## Progression
- 3 procedural but straightforward maps
	- essentially tutorialized introduce concepts
	- bad ending
- primary sequence has an "escape hatch"
	- maybe based on a particular use of items
	- breaks into a secondary set of levels that eschew the established rules

## Programs
How do these fit into the theme of "Breaking the Code".

* GOTO: Move from one location to a new random one.
* PEEK: Shows enemies.
* NOP SLIDE: Change sections of the map to all open squares.
* ESC SEQ: Disconnect from the grid for an amount of time.
* CLIP: Clip through a wall.

## Thoughts

Cron's control flow could be expanded to spawn new tasks if desired.
A fire-and-forget animation system would be nice. Different from Cron in that it's explicitly timed things.

## Interaction

Control scheme: 
Movement: Up/Down/Left/Right, WASD, JKL;,
Context switch: everything else

Programs extend from user in circular menu, rotating to show movement works.

## TODO

* audio
	* bad end hum
* fix overlay w/ sparkle and line rendering
* player state during transition
* enemies
	* include a "program" to help deal with them
		* I think this means more avoidance than destruction.
* aesthetic shader per map changes
* finish bad ending
	* four sentinels
	* "explosion" shader
	* grayscale map
