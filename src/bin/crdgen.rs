use kube::CustomResourceExt;

fn main() {
    println!("{}", serde_yaml::to_string(&habitat::api::Job::crd()).unwrap());
}
