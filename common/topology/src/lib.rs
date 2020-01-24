use itertools::Itertools;
use rand::seq::IteratorRandom;
use sphinx::route::Node as SphinxNode;
use std::cmp::max;
use std::collections::HashMap;
use version_checker;

pub mod coco;
pub mod mix;
pub mod provider;

pub trait NymTopology: Sized {
    fn new(directory_server: String) -> Self;
    fn new_from_nodes(
        mix_nodes: Vec<mix::Node>,
        mix_provider_nodes: Vec<provider::Node>,
        coco_nodes: Vec<coco::Node>,
    ) -> Self;
    fn mix_nodes(&self) -> Vec<mix::Node>;
    fn providers(&self) -> Vec<provider::Node>;
    fn coco_nodes(&self) -> Vec<coco::Node>;
    fn make_layered_topology(&self) -> Result<HashMap<u64, Vec<mix::Node>>, NymTopologyError> {
        let mut layered_topology: HashMap<u64, Vec<mix::Node>> = HashMap::new();
        let mut highest_layer = 0;
        for mix in self.mix_nodes() {
            // we need to have extra space for provider
            if mix.layer > sphinx::constants::MAX_PATH_LENGTH as u64 {
                return Err(NymTopologyError::InvalidMixLayerError);
            }
            highest_layer = max(highest_layer, mix.layer);

            let layer_nodes = layered_topology.entry(mix.layer).or_insert(Vec::new());
            layer_nodes.push(mix);
        }

        // verify the topology - make sure there are no gaps and there is at least one node per layer
        let mut missing_layers = Vec::new();
        for layer in 1..=highest_layer {
            if !layered_topology.contains_key(&layer) {
                missing_layers.push(layer);
            }
            if layered_topology[&layer].len() == 0 {
                missing_layers.push(layer);
            }
        }

        if missing_layers.len() > 0 {
            return Err(NymTopologyError::MissingLayerError(missing_layers));
        }

        Ok(layered_topology)
    }

    // Tries to get a route through the mix network
    fn mix_route(&self) -> Result<Vec<SphinxNode>, NymTopologyError> {
        let mut layered_topology = self.make_layered_topology()?;
        let num_layers = layered_topology.len();
        let route = (1..=num_layers as u64)
            .map(|layer| layered_topology.remove(&layer).unwrap()) // for each layer
            .map(|nodes| nodes.into_iter().choose(&mut rand::thread_rng()).unwrap()) // choose random node
            .map(|random_node| random_node.into()) // and convert it into sphinx specific node format
            .collect();

        Ok(route)
    }

    // Sets up a route to a specific provider
    fn route_to(&self, provider_node: SphinxNode) -> Result<Vec<SphinxNode>, NymTopologyError> {
        Ok(self
            .mix_route()?
            .into_iter()
            .chain(std::iter::once(provider_node))
            .collect())
    }

    fn all_paths(&self) -> Result<Vec<Vec<SphinxNode>>, NymTopologyError> {
        let mut layered_topology = self.make_layered_topology()?;
        let providers = self.providers();

        let sorted_layers: Vec<Vec<SphinxNode>> = (1..=layered_topology.len() as u64)
            .map(|layer| layered_topology.remove(&layer).unwrap()) // get all nodes per layer
            .map(|layer_nodes| layer_nodes.into_iter().map(|node| node.into()).collect()) // convert them into 'proper' sphinx nodes
            .chain(std::iter::once(
                providers.into_iter().map(|node| node.into()).collect(),
            )) // append all providers to the end
            .collect();

        let all_paths = sorted_layers
            .into_iter()
            .multi_cartesian_product() // create all possible paths through that
            .collect();

        Ok(all_paths)
    }

    fn filter_node_versions(
        &self,
        expected_mix_version: &str,
        expected_provider_version: &str,
        expected_coco_version: &str,
    ) -> Self {
        let mixes = Filter::new(self.mix_nodes()).run(expected_mix_version);
        let providers = Filter::new(self.providers()).run(expected_provider_version);
        let cocos = Filter::new(self.coco_nodes()).run(expected_coco_version);

        Self::new_from_nodes(mixes, providers, cocos)
    }

    fn filter<T: Versioned>(&self, nodes: Vec<T>, expected_version: &str) -> Vec<T> {
        nodes
            .iter()
            .filter(|node| version_checker::is_compatible(&node.get_version(), expected_version))
            .cloned()
            .collect()
    }

    fn can_construct_path_through(&self) -> bool {
        match self.make_layered_topology() {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

pub trait Versioned: Clone + Sized {
    fn get_version(&self) -> String;
}

pub struct Filter<T> {
    nodes: Vec<T>,
}

impl<T: Versioned> Filter<T> {
    fn new(nodes: Vec<T>) -> Self {
        Self { nodes }
    }

    fn run(&self, expected_version: &str) -> Vec<T> {
        self.nodes
            .iter()
            .filter(|node| version_checker::is_compatible(&node.get_version(), expected_version))
            .cloned()
            .collect()
    }
}

#[derive(Debug)]
pub enum NymTopologyError {
    InvalidMixLayerError,
    MissingLayerError(Vec<u64>),
}
