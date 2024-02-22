# Adapting the Taiko zkevm prover for Gevulot


## 1. Overview

In this tutorial (or case study), we will show how to adapt an existing prover so that it may be deployed on a Gevulot node.  To illustate the steps involved, we will use the Taiko zkevm prover currently used on the Katla testnet (alpha 6).  This repository contains forks of their `zkevm-chain` and `zkevm-circuits` packages.

Taiko uses a Halo2-based prover for its L2 rollups.

Our goal is to take you through all the steps to get this prover executing in Gevulot.

- we describe how to create a witness required for the prover
- how we created standalone prover and verifier binaries
- how we adapted the binaries to run in the Gevulot environment

This tutorial is meant to be run from the `zkevm-chain` folder.  The `taiko-demo` repository itself, should be installed in the same folder (parallel to) as the [Gevulot](https://github.com/gevulotnetwork/gevulot) repository (for linking to the `gevulot_shim` library).


## 2. The Prover


The Taito prover resides in this repository: https://github.com/taikoxyz/zkevm-chain

Currently, Taiko are running their Katla testnet (Alpha 6).  Therefore, we have used  the `v0.6.0-alpha` branch.  For illustration purposes, this particular commit:

- https://github.com/taikoxyz/zkevm-chain/tree/275eec0097400241ab71963f6c1a192019d219cb


There are three phases to this the code changes we have to do:
1. Adapt the `prover_cmd` binary to support witness capture, offline proving and verification
2. Create a new binary for the prover which we will package, as well as one for the verifier
3. Package the binaries as Nanos unikernel images.


## 3 Adapt `prover_cmd`

The original `prover_cmd` binary (whose main function is [here](https://github.com/taikoxyz/zkevm-chain/blob/v0.6.0-alpha/prover/src/bin/prover_cmd.rs#L12)) is a convenient point of entry for us into the Taiko prover.  While the Taiko provers are normally instantiated via RPC requests, the functionality is also exposed here as a standalone binary.  Three parameters are passed in via the environment:
- a block number
- an RPC endpoint for the L2 node
- a proof parameters file

The basic outline of what we need to do is to serialize a witness, in conjunction with a live L1/L2 node setup.  Once we have that, then we can adapt the prover to use that file, eliminating the need for an online connection.


The changes we have made may be summarized here:
- introduce a prover mode enum parameter to enable different behaviors:
  - witness capture: with a block number and RPC url, grab the witness and serialize it to a file
  - offline prover: given  a witness and proof params file, generate a proof and write it to disk
  - legacy prover: this operates the same as prover_cmd originally did
  - verifier: take a proof and verify it
- use command line arguments instead of environment variables


#### Download the proof parameters file

In order to do any kind of proving, we will first need a 512MiB proof parameters file. The way the examples are set up here, we expect to find it in the `gevulot` folder

While the `zkevm-chain` folder, run this:

```
wget -P ./gevulot https://storage.googleapis.com/zkevm-circuits-keys/kzg_bn254_22.srs
```

### 3.1 Witness capture

A circuit witness forms the basis of what gets proven in this protocol. The specific data here are gathered from querying the RPC node for blocks, block transactions, and code.  These are some of the specific calls that get made:

```
eth_getBlockByNumber
eth_getBlockByHash
debug_traceBlockByNumber
eth_getCode
eth_getProof
```

The first step in getting this to work was to add `Serialize` and `Deserialize` traits to  the `CircuitWitness`.  This necessitated adding serialization to the component structs in the `zkevm-circuits` package as well.  We have forked that, here in repository next to `zkevm-chain`.  The prover `Cargo.toml` file links to that library.

The code that serializes the witness, writing it out is here: 
https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/shared_state.rs#L667-L671


In order to capture a witness, the arguments are: 
- `-b` :  block number
- `-k` :  proof parameters files: `gevulot/kzg_bn254_22.srs`
- `-r` :  RPC endpoint, e.g. `http://35.205.130.127:8547`
- `-w` :  output witness file


If you have access to a Katla L2 node RPC endpoint, you can go ahead and create a witness. An example
```
./target/release/prover_cmd witness_capture -b 57437 -k gevulot/kzg_bn254_22.srs -r http://35.205.130.127:8547 -w witness.json
```


### 3.2 Offline prover

The arguments are:
- `-k` :  proof parameters file: `gevulot/kzg_bn254_22.srs`
- `-w` :  input witness file
- `-p` :  output proof file



Running it can take some time, depending on system resources.

```
./target/release/prover_cmd offline_prover -k gevulot/kzg_bn254_22.srs -w gevulot/witness-57437.json  -p proof.json
```


### 3.3 Verifier and legacy prover

We have exposed a verifier mode, which is normally not done separately by `prover_cmd`, but rather at the end of the proof generation as a check.  We have encapsulated that code for our verifier.  There is also verification done on-chain, by the L1 node. 

Additionally, we support the legacy prover, which uses a live RPC connection to generate the witness, being used directly to generate the proof. We use command line arguments here, instead of environment variables used in the original version.

### 3.4 Summary

The four modes of `prover_cmd` are illustrated with the following calls.  The witness capture and legacy prover both require a live RPC Katla endpoint.  They should all work as written, given a valid connection.

```
./target/release/prover_cmd witness_capture -b 57437 -k gevulot/kzg_bn254_22.srs -r http://35.205.130.127:8547 -w witness.json
./target/release/prover_cmd offline_prover -k gevulot/kzg_bn254_22.srs -w gevulot/witness-57437.json -p proof.json
./target/release/prover_cmd legacy_prover -b 57437 -k gevulot/kzg_bn254_22.srs -r http://35.205.130.127:8547 -p proof.json
./target/release/prover_cmd verifier -p proof.json
```


## 4. Constraints

Now that we have a binary that can run with four different actions, we can try to run them as an Nanos unikernel.  This section deals with two constraints to bear in mind when building or adapting a prover for use with Gevulot
1. you may not fork another process
2. you might not have permission for writing to the root directory

Additionally, our shim require the use of a non-async main() function.  Any async calls must be adapted or rewritten. This will be covered in the section on creating unikernel images.


### 4.1 Run `prover_cmd` as a unikernal

First, we now need to know how to run a binary as a unikernel.  

If `ops` is not yet installed, do it now

```
sudo apt-get install qemu-kvm qemu-utils
```

Next, set up a volume.

```
mkdir ops_deploy
ops volume create taiko_deploy -n -s 2g -d ops_deploy
```

You may now run unikernel images.  For example, run the verifier.  The arguments to `prover_cmd` have been set in the manifest file.

```
ops run ./target/release/prover_cmd -n -c gevulot/taiko-ops-verifier.json --mounts taiko_deploy:/ops_deploy
```
You should see an `Ok` in the last two lines
```
deploy code size: 21884 bytes, instances size: [1][18], calldata: 4384
gevulot_evm_verify result: Ok(604658)
```

We have set up another manifest that points to a 'bad` proof, namely, a proof file where one random byte has been altered.  This will return an error.

```
ops run ./target/release/prover_cmd -n -c gevulot/taiko-ops-verifier-fail.json --mounts taiko_deploy:/ops_deploy
```



### 4.2 No forked processes

As part of running the original Taiko prover, a Solidity script must be compiled.   In the commented out line [here](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/shared_state.rs#L357-L358), a call the the solidity compiler executable `solc` happens on [this line](https://github.com/taikoxyz/snark-verifier/blob/main/snark-verifier/src/loader/evm/util.rs#L105).

First, try running the offline prover via ops.  Running on a 32GB system if prefered, although we have run it on a 16GB laptop with 32GB of swap space.  It will write out the file `proof.json` to `zkevm-chain`, taking six minutes (or longer) to run.

```
ops run ./target/release/prover_cmd -n -c gevulot/taiko-ops-offline-prover.json --mounts taiko_deploy:/ops_deploy --smp 16
```

When we comment that line in, as well as the import statement on line 25 of shared_state.rs, rebuild and run the offline prover as we just have, we will get an error that looks like this:

```
thread 'tokio-runtime-worker' panicked at /home/ader/.cargo/git/checkouts/snark-verifier-79f3a4e94e319a00/612f495/snark-verifier/src/loader/evm/util.rs:118:13:
Failed to spawn cmd with command 'solc':
Operation not permitted (os error 1)
```

In this particular case, the work-around was not so simple. We had to:
- build the [Solidity compiler library](https://github.com/ethereum/solidity) (C++) 
- link the libraries to the prover executable.
- import an external function from the library, and call it from Rust.


After we built (and slightly modified) the library, we added the static libs here here:  https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/build.rs#L54-L68

We have included the required static libraries as part of this package.  You may have to adjust the paths, depending are where some of the standard libraries may be located.

The call to `gevulot_compile` is made from the `local_compile_solidity` function.

### 4.3 Do not write to `./`

Another problem we found was with the default behavior of the `gen_verifier` function.

That happened [here](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-circuits/circuit-benchmarks/src/taiko_super_circuit.rs#L93), where a solidity script gets written out under the name of `aggregation_plonk.sol` 


If we comment that line in (and again, adjust the imports at line 23), we'll get the following error when running the prover:
```
thread 'tokio-runtime-worker' panicked at /home/ader/dev/gev/taiko-demo/zkevm-circuits/circuit-benchmarks/src/taiko_super_circuit.rs:91:59:
called `Result::unwrap()` on an `Err` value: Os { code: 28, kind: StorageFull, message: "No space left on device" }
```

File writing may only be done to specific paths.


## 5. Creating the prover and verifier images

### 5.1 Overview

The runtime environment in which the provers run have very specfic structure for `main()` and the run tasks.

The signature of main [here in the mock prover](), looks like this

```
fn main() -> Result<(), Box<dyn Error>>
```

We have added three binary executables to the prover package
- [a prover](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/bin/taiko_prover.rs)
- [a verifier](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/bin/taiko_verifier.rs)
- [a mock prover](https://github.com/gevulotnetwork/taiko-demo/blob/main/zkevm-chain/prover/src/bin/taiko_mock.rs)

We link also to the `gevulot_shim` library.


Build them
```
cargo build --release
```

### 5.2 Build the images

An ops image is created from a binary and a manifest, which may also include other static files.  In this use case, our 512MiB parameters file is part of the package.

```
ops build ./target/release/taiko_prover -n -c ./gevulot/manifest_prover.json && \
ops build ./target/release/taiko_verifier -n -c ./gevulot/manifest_verifier.json && \
ops build ./target/release/taiko_mock -n -c ./gevulot/manifest_mock.json
```

We are now ready to run the images on Gevulot!

## 6. Executing the prover on Gevulot

You now should have installed the [gevulot repository](https://github.com/gevulotnetwork/gevulot), parallel to the `taiko-demo` folder. We will now be working from there.


### 6.1 Prerequites

Here is the [Gevulot installation guide](https://github.com/gevulotnetwork/gevulot/blob/main/INSTALL.md).

You will need to obtain two keys for whitelisting later, namely, the local key and the node node.
To display them, you will use the `gevulot show` command. We have built a debug

```
$ ./target/debug/gevulot show public-key --key-file /var/lib/gevulot/node.key
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
cargo sqlx database drop --database-url postgres://gevulot:gevulot@localhost/gevulot  (type `y` to confirm)
cargo sqlx database create --database-url postgres://gevulot:gevulot@localhost/gevulot
cargo sqlx migrate run --database-url postgres://gevulot:gevulot@localhost/gevulot
cd ../..
```

#### 6.2.3 Whitelist your keys

Following a database initialization, you must whitelist your keys. Those strings were obtained with the `show` command above.  Back in the project root, run:

```
./target/debug/gevulot peer 04715a75faf7407de5a627a8cafb325e8abe146dfe4a1255... whitelist
./target/debug/gevulot peer 042bd568e378a3b71a97e867f82131b849fdfa271f0fc623... whitelist
```


#### 6.2.4 Launch the node

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

The deployment step registers a prover and verifier, being unikernel images. They must be registered together, as a pair.

```
gevulot-cli deploy --name taiko-zkevm --prover /home/ader/.ops/images/taiko_prover --verifier /home/ader/.ops/images/taiko_verifier
```

The output will include hash values that should be copied.

```
Start prover / verifier deployment
  [00:00:02] [##########################################################] 25.54 MiB/25.54 MiB (taiko_verifier-0.0s)
  [00:00:02] [##########################################################] 563.69 MiB/563.69 MiB (taiko_prover-0.0s)Prover / Verifier deployed correctly.
Prover hash:bcaf4dcc5408f9fa1eadbe80163c1bd0e20e41ce2407ee1601b61bfa4cff3112
Verifier hash:62ed37dfff36e7a5fd335b4d4fc3b3c27a2c624c5a1034efbf15ee11384b1d10.
Tx Hash:fbb7df66a50610c89e2fbb70684e89a881d332db599080db6a1650022b5268ad
```

Copy the prover and verifier hash strings -- they will be used in the next step.

### 6.3 Task execution

This example shows how to execute the prover.

```
$ ./target/debug/gevulot-cli exec --tasks '[{"program":"bcaf4dcc5408f9fa1eadbe80163c1bd0e20e41ce2407ee1601b61bfa4cff3112","cmd_args":[{"name":"-k","value":"kzg_bn254_22.srs"}, {"name":"-p","value":"/workspace/proof.json"}, {"name":"-w","value":"/workspace/witness-57437.json"}],"inputs":[{"Input":{"file":"witness-mock.json"}}]},{"program":"62ed37dfff36e7a5fd335b4d4fc3b3c27a2c624c5a1034efbf15ee11384b1d10","cmd_args":[{"name":"-p","value":"/workspace/proof.json"}],"inputs":[{"Output":{"source_program":"bcaf4dcc5408f9fa1eadbe80163c1bd0e20e41ce2407ee1601b61bfa4cff3112","file_name":"/workspace/proof.json"}}]}]'
```

There are a few things to note here:
- our parameter file (`kzg_bn254_22.srs`) will be found in the image
- the proof file gets written to the `/workspace` folder
- the inputs to the verifier are based on the output from the prover, our proof file.






