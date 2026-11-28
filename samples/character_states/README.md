# Character animation states

The automatic intro demonstrates idle (red), run (green), a one-shot action (blue),
return to idle, then run. A small blue beacon confirms the full state sequence.
Press Left/Right or A/D to move and take control; Space plays the one-shot action.
This demonstrates animation state control, not a physics jump/controller.

Select Character and open Inspector → Animation States to edit states, numeric
parameters, ordered conditions and finish transitions. Apply commits an undoable
component edit. Replace the tiny color textures/clips with character artwork.
Scripts set parameters and read state; no repeated play-animation requests needed.
