# Cosmic Garden AI Language

```cgai
ai_tick {
    if behavior is Aggressive {
        scan room.who and if [vct] {
            signal Attack for Life with [self] vs [vct];
            wait for [reply];
            if [reply] is negative then repeat @scan
        }
    }

    when state is Idle {

    }

    when state is Wandering {

    }
}
```
