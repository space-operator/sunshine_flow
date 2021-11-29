use sunshine_flow_core::{Command, Config, Flow};

async fn main() {
    let flow = Flow::new(Config {
        commands: vec![],
        url: "devnet url".into(),
        keyring: HashMap::new(),
        pub_keys: HashMap::new(),
    })
    .unwrap();

    flow.run().unwrap();
}
