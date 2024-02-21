# Taiko ZKEVM: a sample prover for Gevulot
**[in progress]**


## 1. Overview

In this document, we will show how to adapt an existing prover so that it may be deployed on a Gevulot node.  To illustate the steps involved, we will use the Taiko zkevm prover currently used on the Katla testnet (alpha 6).  This repository contains forks of their `zkevm-chain` and `zkevm-circuits` packages.


Taiko uses a Halo2-based prover for its L2 rollups.

The goal of this document is to take you through all the steps to get this prover executing in Gevulot.

- we describe how create a witness required for the prover
- how we created standalone prover and verifier binaries.
- how we adapt the binraries to run in the Gevulot environment
- furthermore, it's a nice juicy prover tasks:  it takes about 6 minutes to run, using 22 cores and 17 GB of memory
- we can examine the proofs written to the L1 node, which is Holesky in this case



## 2. The Prover

- Description of the Taiko prover

The Taito prover reside in this repository:

As currently, alph6 is running on their Katla testnet, we have used  the `v0.6.0-alpha` branch, and for illustration this particular commit:


https://github.com/taikoxyz/zkevm-chain/tree/275eec0097400241ab71963f6c1a192019d219cb

point to various places in the code to illustrate
- how the environment args are parsed in prover_cmd
- what line of code creates the witnes
- where the proof is generated, with a bit of stack
- where the verifier is called


There are 3 main parts to this coding process
1. Adapt the `prover_cmd` binary to support witness capture, offline proving and verification
2. Create a new binary for the prover which we will package, as well as one for the verifier
3. Package these both as Nanos unikernel images.


## 3 Adapt `prover_cmd`.

The original `prover_cmd` binary (whose main function is [here](https://github.com/taikoxyz/zkevm-chain/blob/v0.6.0-alpha/prover/src/bin/prover_cmd.rs#L12)) is a convenient point of entry for us into the Taiko prover.  While the Taiko provers are normally instantiated vai RPC requests, the functionality is more or less exposed here in a standalone executable.  Three parameters are passed in via the environment:
- a block number
- an RPC endpoint for the L2 node
- a proof parameters file

The basic outline of what we need to do is to serialize a witness, in conjunction with a live L1/L2 node setup.  Once we have that, then we can adapt the prover to use a passed in file, which eliminates the need for an online connection.


The changes we have made in this executable may be summarized here:
- introduce a prover mode enum parameter to enable different behaviors
  - witness capture: with a block number and RPC url, grab the witness and serialize it to a file
  - offline prover: with a witness and proof params file, generate a proof and write it to disk
  - legacy prover: this operates the same as prover_cmd originally did
  - verifier: receive a proof and verify it
- use command line arguments instead of environment variables


#### Download the proof parameters file

In order to do any kind of proving, you will first need a 512MiB proof parameters file. The way the examples are set up here, we expect to find it in the `./zkevm-chain/gevulot` folder

While at the root of the `zkevm-chain` package, run this:

```
wget -P ./gevulot https://storage.googleapis.com/zkevm-circuits-keys/kzg_bn254_22.srs
```

### 3.1 Witness capture

We have rewritten the prover_cmd binary, the main function originally here:

The first thing we did was exchange the arguments passed in as environment variables for command line parsing.

We also defined a prover mode enum, for the four actions we will eventually support:
- create a witness for a given block number
- create a proof from a witness
- perform a verification from a proof
- there's also a legacy prover, that takes a block number and creates a proof directly.


The data here is gathered from querying the node for blocks, block transactions, and code.  These are the specific calls that get made


This part of the proof takes as input an L2 block number.  In the version is `zkevm-chain` we are using, that is a Katla testnet node.

```
eth_getBlockByNumber
eth_getBlockByHash
debug_traceBlockByNumber
eth_getCode
eth_getProof
```



We will use the prover_cmd.

#### Witness Serialization

We have to add the Deserialize and Serialize traits for the 

#### Write it out


### 3.2 Offline prover

The arguments are...
- `-k` :  ...


We have exposed the legacy prover, as well as a verifier

 >The code may be built and run. For witnes cap...

In order to capture a witness, pass in: 
- a block number, 
- parameters file path, usually kzg_bn254_22.srs
- an RPC endpoint.  This is the L2 Katla node that you are running.
- 
./target/release/prover_cmd witness_capture -k kzg_bn254_22.srs -w witness-57437.json
./target/release/prover_cmd offline_prover -k kzg_bn254_22.srs -w witness-57437.json  -p proof.json
./target/release/prover_cmd legacy_prover -b 57437 -k kzg_bn254_22.srs -r http://35.195.113.51:8547 -p proof-legacy-57437.json
./target/release/prover_cmd verifier -k -p proof.json


### 3.3 `prover_cmd` with nanos.

## 4. Constraints

There are a few things to bear in mind when building or adapting a prover for use with Gevulot
- you may not fork another process
- you might not have permission for writing to the root directory
- you do not have access to the network

Additionally, the shim require the use of a non-async main() function.  Any async calls must be adapted or rewritten. This will be covered in the section on creating unikernel images.


### 4.1 No forked processes

As part of running the original Taiko prover, a Solidity script must be compiled.   In the commented out line [here](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/shared_state.rs#L357-L358), a call the the solidity compiler executable `solc` happens on [this line](https://github.com/taikoxyz/snark-verifier/blob/main/snark-verifier/src/loader/evm/util.rs#L105).

When we comment that line in, as well as the import statement on lin 25 of shared_state.rs, and run the offline prover as we just have, we will get an error that looks like this:

```
thread 'tokio-runtime-worker' panicked at /home/ader/.cargo/git/checkouts/snark-verifier-79f3a4e94e319a00/612f495/snark-verifier/src/loader/evm/util.rs:118:13:
Failed to spawn cmd with command 'solc':
Operation not permitted (os error 1)
```

In this particular case, the work-around was not so simple:
- build the solidity compiler library (C++, ) 
- link the libraries to the prover executable.
- import an external function from the library, and call it from Rust.


The build script was modified here:  https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/build.rs#L54-L68

We have included the required static libraries as part of this package.  You may have to adjust the paths, depending are where some of the standard libraries may be located.

The call to `gevulot_compile` is made from the `local_compile_solidity` function.

### 4.2 Do not write to `./`

Another problem we found was with the default behavior of the `gen_verifier` function.

That happened here, where a solidity script gets written out under the name of `aggregation_plonk.sol` 
https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-circuits/circuit-benchmarks/src/taiko_super_circuit.rs#L93

If we comment that line in (and again, adjust the imports at line 23), we'll get the following error when running the prover:
```
thread 'tokio-runtime-worker' panicked at /home/ader/dev/gev/taiko-demo/zkevm-circuits/circuit-benchmarks/src/taiko_super_circuit.rs:91:59:
called `Result::unwrap()` on an `Err` value: Os { code: 28, kind: StorageFull, message: "No space left on device" }
```

File writing may only be done to specic paths, which will be covered later.

### 4.3 No networking





## 5. Creating the prover and verifier images

We have added two binary executables to the prover package

One for the prover: `<link to main() in taiko_prover.rs>
One for the verifier: `<link to main() in taiko_verifier.rs>


## 5.1 Build the binaries

define the API

## 5.2 Run them as ops images



## 6. Executing the prover


### 6.1 Prerequites

Link to installation

Obtain the local key and node key.  These 

You will need to obtain two keys for whitelisting later, namely, the local key and the node node.
To display them, you will use the `gevulot show` command.  

```
$ gevulot show public-key --key-file /var/lib/gevulot/node.key 
042bd568e378a3b71a97e867f82131b849fdfa271f0fc6238ef...
$ ./target/debug/gevulot show public-key --key-file localkey.pki 
04715a75faf7407de5a627a8cafb325e8abe146dfe4a1255963...
```

### 6.2 Running the node

#### 6.2.1 Start up the database

In a fresh terminal, our first
```
cd crates/node
podman-compose up
```

#### 6.2.2 Reinitialize the database

While testing, it is often a good idea to start with a clean database.  Perform a manual reinitialization in a second terminal instance

```
cd crates/node
cargo sqlx database drop --database-url postgres://gevulot:gevulot@localhost/gevulot
cargo sqlx database create --database-url postgres://gevulot:gevulot@localhost/gevulot
cargo sqlx migrate run --database-url postgres://gevulot:gevulot@localhost/gevulot
```

#### 6.2.3 Launch the node

This is a possible command to launch the node.  Here, I've specified
- the debug executable
- various logging options
- a data directory path, overriding the default (which is `/var/lib/gevulot`).

```
RUST_LOG=warn,gevulot=debug,sqlx=error ./target/debug/gevulot run --data-directory ~/.gev
```

Typically, you should see initial output like this:

```
2024-02-15T16:50:50.754591Z DEBUG gevulot::networking::p2p::pea2pea: new tx handler registered
2024-02-15T16:50:50.754695Z  INFO gevulot::networking::download_manager: listening for http at 127.0.0.1:9995
2024-02-15T16:50:50.791134Z  INFO gevulot: listening for p2p at 127.0.0.1:9999
2024-02-15T16:50:50.791931Z  INFO gevulot::rpc_server: listening for json-rpc at 127.0.0.1:9944
```

### 6.3 Deployment

The deployment step registers a prover and verifier, being unikernel images.

You must register a prover and a verifier together, as a pair.

```
gevulot-cli deploy --name taiko-zkevm --prover /home/ader/.ops/images/taiko_prover --verifier /home/ader/.ops/images/taiko_verifier
```

The output will include hash values that should be copied and saved, 

```
$ gevulot-cli deploy --name taiko-zkevm --prover /home/ader/.ops/images/taiko_prover --verifier /home/ader/.ops/images/taiko_verifier
Start prover / verifier deployment
  [00:00:02] [##########################################################] 25.54 MiB/25.54 MiB (taiko_verifier-0.0s)
  [00:00:02] [##########################################################] 563.69 MiB/563.69 MiB (taiko_prover-0.0s)Prover / Verifier deployed correctly.
Prover hash:bcaf4dcc5408f9fa1eadbe80163c1bd0e20e41ce2407ee1601b61bfa4cff3112
Verifier hash:62ed37dfff36e7a5fd335b4d4fc3b3c27a2c624c5a1034efbf15ee11384b1d10.
Tx Hash:fbb7df66a50610c89e2fbb70684e89a881d332db599080db6a1650022b5268ad
```

### 6.3 Task execution







