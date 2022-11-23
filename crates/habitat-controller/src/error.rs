use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Finalizer Error: {0}")]
    FinalizerError(#[source] kube::runtime::finalizer::Error<kube::Error>),

    #[error("Kube Error: {0}")]
    KubeError(#[source] kube::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
