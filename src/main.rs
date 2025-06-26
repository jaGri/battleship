fn main() {

    
    println!("Hello, world!");
}




// mod domain; mod engine_api; mod engine; mod protocol;
// mod transport; mod stub; mod skeleton; mod cli;

// use transport::InMemoryTransport; use engine::Engine;
// use stub::Stub; use skeleton::Skeleton; use cli::run_cli;
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     // choose transport
//     let (t1, t2) = InMemoryTransport::new_pair();
//     let mut skel1 = Skeleton { engine: Engine::new(), transport: t1.clone() };
//     tokio::spawn(async move { skel1.run().await.unwrap() });

//     let api: Box<dyn GameApi> = Box::new(Stub::new(t2));
//     run_cli(api).await
// }