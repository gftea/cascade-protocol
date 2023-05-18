# cascade-protocol
Rust implementation for Cascade information reconciliation protocol for Quantum Key Distribution (QKD)

The work is based on [Cascade-CPP](https://github.com/brunorijsman/cascade-cpp) and [The Cascade information reconciliation protocol](https://cascade-python.readthedocs.io/en/latest/index.html)

# Current status of the implementation

- Only implement the classical cascade protocol
- Focus on testing efficiency and correctness of the algorithm, demonstrating possible limitation
- It stubs away the surrounding dependencies, e.g. communication and parallel computing


# Improvement ideas

- when the block size is 2, and we get even error parity, then there could only two cases:
<pre>
   |correct  |error
   |-------- |--------
   | 0, 0    | 1, 1
   |-------- |--------
   | 0, 1    | 1, 0
</pre>

    Instead of just stop here, one improvement to original algorithm can be to ask for parity of one bit and if it is different, flip all two bits. It may introduce one additional communication, but 
    could potentially correct 2 bits.
    