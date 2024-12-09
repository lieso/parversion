
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisNetwork {
    pub id: String,
    pub description: String,
    pub associations: Option<HashSet<SubgraphHash>>
}

