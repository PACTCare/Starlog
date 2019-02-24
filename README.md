<img src="https://pact.online/dist/img/starlog.png" width="200">

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

> IPFS Metadata Blockchain based on Substrate

The goal of Starlog is to research and develop an open source solo chain or potential parachain, which stores metadata for IPFS as non-fungible tokens. The metadata will be signed by the uploaders and include availability information, a price, a timestamp, a license, information about the uploaded file itself as well as the location/gateway of the initial upload.

Therefore, Starlog provides the following key benefits for IPFS uploads:

- Searchability via human-readable names
- Clear ownership for IPFS uploads
- Marketplace for uploaded content
- Electronic identity based on personal IPFS uploads
- Faster initial loading of non-distributed IPFS content

Furthermore, the combination of IPFS and Substrate seems to be a potential scalable and relatively cheap solution for all kinds of distributed applications. 

The project was initiated to improve [Dweb.page](https://github.com/PACTCare/Dweb.page). 

---

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Maintainer](#maintainer)
- [Contributing](#contributing)
- [License](#license)

## Install

If you haven’t installed [Substrate](https://www.parity.io/substrate/) before, check out the official [Substrate documentation](https://substrate.readme.io/docs/getting-started). If you are a windows user, I also suggest taking a look at the [Substrate GitHub](https://github.com/paritytech/substrate).

## Usage

Clone this repository and run the following commands:
```
./build.sh
cargo build --release
./target/release/starlog purge-chain --dev
./target/release/starlog --dev
```

You can interact with your local chain using the [Polkadot-JS Apps UI](https://polkadot.js.org/apps/).
Therefore, you need to adjust the Settings:
```
Remote node/endpoint to connect to > Local Node (127.0.0.1:9944)
Default Interface Theme > Substrate
```

Under the developer tab upload and save the [Metadata.json](https://github.com/PACTCare/Starlog/blob/master/Metadata.json) to register the custom struct. 

## Maintainer

[David Hawig](https://github.com/Noc2)

## Contributing

If you want to help either join our **[discord server](https://discord.gg/VMj7PFN)** or you can open issues for bugs you've found or features you think are missing. You can also submit pull requests to this repository.

If editing the README, please conform to the [standard-readme specification](https://github.com/RichardLitt/standard-readme).

## License
[MIT License](https://github.com/PACTCare/Starlog/blob/master/LICENSE) © PACT Care B.V.