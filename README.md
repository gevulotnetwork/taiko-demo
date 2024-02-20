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


## 3 Adapt the prover.

Here we will describe all the changes in the code required to get to where we want to be. We have to rewrite parts of the prover to do two things:
1. Stop the process once the witness is generated
2. Serialize the witness and write it to a file path that gets passed in.

Additionally, we like to 
- pass in a witness file and generate a proof from that
- pass in a proof and perform the verfication.

We will do this in four steps
1. implement witness capture, k
2. enable offline proving and verification
3. package the prover and verifier binaries to be used in a unikernel, and run them.  Additionally, we fix any problems that emerge (spoiler alert: one or two do!)
4. build and run the unikernel, fixing any problems that emerge (spoiler alert: one or two do!)

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
- may not have write permission to the root

Additionally, the shim require the use of a non-async main() function.  Any async calls must be adapted or rewritten. This will be covered in the section on creating unikernel images.





## 4. Creating the prover and verifier images

We have added two binary executables to the prover package

One for the prover: `<link to main() in taiko_prover.rs>
One for the verifier: `<link to main() in taiko_verifier.rs>


## 4.1 Build the binaries

define the API

## 4.2 Run them as ops images

## 5. A Problem!


We saw in the last step that our proof did not finish.

### 5.1

We are launched another process in this line




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

### 5.2 Running the node

#### 5.2.1 Start up the database

In a fresh terminal, our first
```
cd crates/node
podman-compose up
```

#### 5.2.2 Reinitialize the database

While testing, it is often a good idea to start with a clean database.  Perform a manual reinitialization in a second terminal instance

```
cd crates/node
cargo sqlx database drop --database-url postgres://gevulot:gevulot@localhost/gevulot
cargo sqlx database create --database-url postgres://gevulot:gevulot@localhost/gevulot
cargo sqlx migrate run --database-url postgres://gevulot:gevulot@localhost/gevulot
```

#### 5.2.3 Launch the node

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

### 5.3 Deployment

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

### 5.3 Task execution







