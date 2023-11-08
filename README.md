# webgpu-math

A GPU accelerated finite field math library.

## Current state

A naive modular multiplication implementation is as fast as the cpu implementation for a batch of 2048 (modular) multiplications.

```
total multiplications: 2048
gpu: 8.68ms
cpu: 9.28ms
```

It's twice as fast for 8192 multiplications:

```
total multiplications: 8192
gpu: 14.06ms
cpu: 27.93ms
```
