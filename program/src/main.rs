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
    tuple(uint32, uint32)
};
pub type H = PoseidonHash;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = <C as GenericConfig<D>>::F;
use plonky2::field::types::Field;

#[derive(Clone, Debug)]
struct PoseidonNode {
    identifier: F,
    value: F,
    left_node: HashOut<F>,
    right_node: HashOut<F>,
}

impl PoseidonNode {
    fn generate_leaves(n: usize) -> Vec<PoseidonNode> {
        (0..n)
            .map(|i| {
                let v = F::from_canonical_usize(i);
                PoseidonNode {
                    identifier: F::from_canonical_usize(42),
                    value: v,
                    right_node: HashOut {
                        elements: [v, v, v, v],
                    },
                    left_node: HashOut {
                        elements: [v, v, v, v],
                    },
                }
            })
            .collect()
    }

    fn generate_tree(leaves: Vec<PoseidonNode>) -> HashOut<F> {
        assert!(leaves.len() % 2 == 0);
        let mut nodes = vec![];
        for node in leaves {
            let varray = [node.identifier, node.value];
            let harray = [node.left_node.elements, node.right_node.elements].concat();
            let mut input = varray.to_vec();
            input.extend(harray);
            let out = H::hash_no_pad(&input);
            nodes.push(out);
        }
        while nodes.len() > 1 {
            let mut to_hash = vec![];
            for i in (0..nodes.len()).step_by(2) {
                if i + 1 == nodes.len() {
                    to_hash.push(*nodes.last().unwrap());
                } else {
                    let h1 = nodes[i];
                    let h2 = nodes[i + 1];
                    let input = [h1.elements, h2.elements].concat();
                    let out = H::hash_no_pad(&input);
                    to_hash.push(out);
                }
            }
            nodes = to_hash;
        }
        *nodes.first().unwrap()
    }
}

type Value = [u8; 32];

struct KeccakNode {
    identifier: Value,
    value: Value,
    left_hash: Value,
    right_hash: Value,
}

impl KeccakNode {
    fn generate_leaves(n: usize) -> Vec<KeccakNode> {
        (0..n)
            .map(|i| {
                let v = [i as u8; 32];
                KeccakNode {
                    identifier: [42 as u8; 32],
                    value: v,
                    left_hash: v,
                    right_hash: v,
                }
            })
            .collect()
    }

    fn generate_tree(leaves: Vec<KeccakNode>) -> Vec<u8> {
        assert!(leaves.len() % 2 == 0);
        let mut nodes = vec![];
        for node in leaves {
            let input = [node.identifier, node.value, node.left_hash, node.right_hash].concat();
            let out = keccak256(&input);
            nodes.push(out);
        }
        while nodes.len() > 1 {
            let mut to_hash = vec![];
            for i in (0..nodes.len()).step_by(2) {
                if i + 1 == nodes.len() {
                    to_hash.push(nodes.last().unwrap().clone());
                } else {
                    let h1 = nodes[i].clone();
                    let h2 = nodes[i + 1].clone();
                    let input = [h1, h2].concat();
                    let out = keccak256(&input);
                    to_hash.push(out);
                }
            }
            nodes = to_hash;
        }
        nodes.first().unwrap().clone()
    }
}
use tiny_keccak::{Hasher as KHasher, Keccak};

fn keccak256(input: &[u8]) -> Vec<u8> {
    let mut k = Keccak::v256();
    let mut output = [0; 32];
    k.update(input);
    k.finalize(&mut output);
    output.to_vec()
}

pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles reading inputs
    // from the prover.
    let n = sp1_zkvm::io::read::<u32>();
    let is_keccak = sp1_zkvm::io::read::<bool>();

    if n > 186 {
        panic!(
            "This fibonacci program doesn't support n > 186, as it would overflow a 32-bit integer."
        );
    }

    // Compute the n'th fibonacci number, using normal Rust code.
    let mut a = 0u32;
    let mut b = 1u32;
    match is_keccak {
        true => {
            let leaves = KeccakNode::generate_leaves(n as usize);
            let tree = KeccakNode::generate_tree(leaves);
        }
        false => {
            let leaves = PoseidonNode::generate_leaves(n as usize);
            let tree = PoseidonNode::generate_tree(leaves);
        }
    }

    let vkeccak = match is_keccak {
        true => 1,
        false => 0,
    };

    // Encocde the public values of the program.
    let bytes = PublicValuesTuple::abi_encode(&(n, vkeccak));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit_slice(&bytes);
}
