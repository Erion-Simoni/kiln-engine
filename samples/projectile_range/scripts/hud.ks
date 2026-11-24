let hits = game_get("hits", 0);
if get_score() < hits { add_score(hits - get_score()); }
