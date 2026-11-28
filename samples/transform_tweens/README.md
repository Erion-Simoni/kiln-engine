# Rotation and scale cutscene

A red actor moves, spins 450 degrees, stretches horizontally and fades to green
while the camera dollies. The automatic sequence pauses all five tracks, checks
held clocks, seeks to midpoint while paused and resumes. The blue beacon confirms
completion. Final rotation turns the stretched actor vertical: green bounds
[192,56,256,184], 8192 pixels; camera origin [32,0].

Space pauses/resumes, Right seeks, R restarts, C cancels and holds. Active tracks
persist in logical game saves. Rotation uses radians without shortest-arc wrapping;
scale is positive XY, supporting squash/stretch. Source Editor authoring; see
[cutscene API](../../docs/CUTSCENE_TWEENS.md). This is not skeletal rigging.
