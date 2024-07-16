//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{sol, SolType};
use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::{GenericConfig, Hasher, PoseidonGoldilocksConfig},
};

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple(uint32, uint32, uint32)
};

pub type H = PoseidonHash;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = <C as GenericConfig<D>>::F;
use plonky2::field::types::Field;

#[derive(Clone, Debug)]
struct Node {
    identifier: F,
    value: F,
}

fn generate_leaves(n: usize) -> Vec<Node> {
    (0..n)
        .map(|i| Node {
            identifier: F::from_canonical_usize(42),
            value: F::from_canonical_usize(i),
        })
        .collect()
}

fn generate_tree(leaves: Vec<Node>) -> HashOut<F> {
    assert!(leaves.len() % 2 == 0);
    let mut nodes = vec![];
    for i in 0..leaves.len() / 2 {
        let n1 = leaves[2 * i].clone();
        let n2 = leaves[2 * i + 1].clone();
        let array = [n1.identifier, n2.value, n2.identifier, n2.value];
        let out = H::hash_no_pad(&array);
        nodes.push(out);
    }
    while nodes.len() > 1 {
        let mut to_hash = vec![];
        for i in (0..nodes.len()).step_by(2) {
            if i + 1 == nodes.len() {
                to_hash.push(*nodes.last().unwrap());
            } else {
                let h1 = nodes[2 * i];
                let h2 = nodes[2 * i + 1];
                let input = [h1.elements, h2.elements].concat();
                let out = H::hash_no_pad(&input);
                to_hash.push(out);
            }
        }
        nodes = to_hash;
    }
    *nodes.first().unwrap()
}
pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles reading inputs
    // from the prover.
    let n = sp1_zkvm::io::read::<u32>();

    if n > 186 {
        panic!(
            "This fibonacci program doesn't support n > 186, as it would overflow a 32-bit integer."
        );
    }

    // Compute the n'th fibonacci number, using normal Rust code.
    let mut a = 0u32;
    let mut b = 1u32;
    let leaves = generate_leaves(n as usize);
    let tree = generate_tree(leaves);

    // Encocde the public values of the program.
    let bytes = PublicValuesTuple::abi_encode(&(n, a, b));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit_slice(&bytes);
}
