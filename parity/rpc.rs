// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.


use std::str::FromStr;
use std::sync::Arc;
use std::net::SocketAddr;
use ethcore::client::Client;
use ethsync::EthSync;
use ethminer::Miner;
use util::RotatingLogger;
use util::keys::store::{AccountService};
use util::network_settings::NetworkSettings;
use die::*;

#[cfg(feature = "rpc")]
pub use ethcore_rpc::Server as RpcServer;
#[cfg(feature = "rpc")]
use ethcore_rpc::{RpcServerError, RpcServer as Server};
#[cfg(not(feature = "rpc"))]
pub struct RpcServer;

pub struct Configuration {
	pub enabled: bool,
	pub interface: String,
	pub port: u16,
	pub apis: String,
	pub cors: Option<String>,
}

pub struct Dependencies {
	pub client: Arc<Client>,
	pub sync: Arc<EthSync>,
	pub secret_store: Arc<AccountService>,
	pub miner: Arc<Miner>,
	pub logger: Arc<RotatingLogger>,
	pub settings: Arc<NetworkSettings>,
}

pub fn new(conf: Configuration, deps: Dependencies) -> Option<RpcServer> {
	if !conf.enabled {
		return None;
	}

	let interface = match conf.interface.as_str() {
		"all" => "0.0.0.0",
		"local" => "127.0.0.1",
		x => x,
	};
	let apis = conf.apis.split(',').collect();
	let url = format!("{}:{}", interface, conf.port);
	let addr = SocketAddr::from_str(&url).unwrap_or_else(|_| die!("{}: Invalid JSONRPC listen host/port given.", url));

	Some(setup_rpc_server(deps, &addr, conf.cors, apis))
}

#[cfg(not(feature = "rpc"))]
pub fn setup_rpc_server(
	_deps: Dependencies,
	_url: &SocketAddr,
	_cors_domain: Option<String>,
	_apis: Vec<&str>,
) -> ! {
	die!("Your Parity version has been compiled without JSON-RPC support.")
}

#[cfg(feature = "rpc")]
pub fn setup_rpc_server(
	deps: Dependencies,
	url: &SocketAddr,
	cors_domain: Option<String>,
	apis: Vec<&str>,
) -> RpcServer {
	use ethcore_rpc::v1::*;

	let server = Server::new();
	for api in apis.into_iter() {
		match api {
			"web3" => server.add_delegate(Web3Client::new().to_delegate()),
			"net" => server.add_delegate(NetClient::new(&deps.sync).to_delegate()),
			"eth" => {
				server.add_delegate(EthClient::new(&deps.client, &deps.sync, &deps.secret_store, &deps.miner).to_delegate());
				server.add_delegate(EthFilterClient::new(&deps.client, &deps.miner).to_delegate());
			},
			"personal" => server.add_delegate(PersonalClient::new(&deps.secret_store).to_delegate()),
			"ethcore" => server.add_delegate(EthcoreClient::new(&deps.miner, deps.logger.clone(), deps.settings.clone()).to_delegate()),
			_ => {
				die!("{}: Invalid API name to be enabled.", api);
			},
		}
	}
	let start_result = server.start_http(url, cors_domain);
	match start_result {
		Err(RpcServerError::IoError(err)) => die_with_io_error(err),
		Err(e) => die!("{:?}", e),
		Ok(server) => server,
	}
}

