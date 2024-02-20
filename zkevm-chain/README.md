
# `prover_cmd`

## Usage

```
Usage: prover_cmd [OPTIONS] <MODE>

Arguments:
  <MODE>  witness_capture | offline_prover | legacy_prover | verifier

Options:
  -b, --block-num <BLOCK_NUM>        Required for witness_capture and legacy_prover
  -r, --rpc-url <RPC_URL>            Url of L2 Taiko node, required for witness_capture and legacy_prover
  -p, --proof-path <PROOF_PATH>      Required for offline_prover and verifier
  -w, --witness-path <WITNESS_PATH>  Required for witness_capture and offline_prover
  -k, --kparams-path <KPARAMS_PATH>  Required for witness_capture, offline_prover, legacy_prover
  -h, --help                         Print help
  -V, --version                      Print version
  ```

There are four prover modes:
- witness capture
- offline prover
- legacy prover
- verifier

## Prerequisites


### Parameters file

Required is a 512MiB proof parameters file, kzg_bn254_22.srs.

That may be gotten thusly:

```
wget https://storage.googleapis.com/zkevm-circuits-keys/kzg_bn254_22.srs <dest>
```

### Solidity compiler

`solc` must be in the path.
It can be built from the sources in this repository:  https://github.com/ethereum/solidity

I used this line for the cmake call:  
```
make .. -DUSE_Z3=OFF`
```
On our cloud server, I copied it into a path folder.
```
sudo cp solc /usr/local/bin
```

## `witness_capture`

Required parameters:
- `-b`: a block number
- `-k`: parameters file with k value of 22. This should be kzg_bn254_22.srs.
- `-r`: an RPC url for the L2 Katla node
- `-w`: witness output file (json)


### Example: create a witness for block 57437

```
./prover_cmd witness_capture -b 57437 -k kzg_bn254_22.srs -r http://35.195.113.51:8547 -w witness.json
```


## `offline_prover`

Required parameters:
- `-k`: parameters file with k value of 22. This should be kzg_bn254_22.srs.
- `-p`: proof output file
- `-w`: witness input file

### Example: create a proof from a witness

```
./prover_cmd offline_prover -k kzg_bn254_22.srs -w witness.json -p proof.json
```


## `legacy_prover`

This is the original mode of operation for `prover_cmd`.  

A witness is created with a connection to an L2 node, followed by the generation of the proof.  We do serialize the proof to a file.


Required parameters:
- `-b`: a block number
- `-k`: parameters file with k value of 22.  This should be kzg_bn254_22.srs.
- `-p`: proof output file
- `-r`: an RPC url for the L2 Katla node

### Example

```
./prover_cmd legacy_prover -b 57437 -k kzg_bn254_22.srs -r http://35.195.113.51:8547 -p proof.json
```

## `verifier`

This mode performs a verification: a proof is read in and verified, with the results written to stdout.

**TBD**


### Example

```
./prover_cmd verfier -p proof.json
```

Required parameters:
- `-p`: proof input file

