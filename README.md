# K8s Resource Timeline

This app watches resource changes (currently secrets, configmaps, pods and nodes) and builds a 
timeline for each resource. Every resource update is displayed as a git diff and categorized 
as metadata / spec / status update.

Run `cargo build --release && target/release/k8s-resource-timeline` and navigate to http://localhost:3000 
to start capturing resource changes using default kubeconfig.

Timeline events can be saved as a JSON file and loaded back in the web interface.
