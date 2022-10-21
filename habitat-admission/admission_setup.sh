#!/usr/bin/env bash
set -euo pipefail

# This script is loosely adapting the TLS setup described in
# https://kubernetes.io/blog/2019/03/21/a-guide-to-kubernetes-admission-controllers/#tls-certificates
# for local development

# Require: a private ip reachable from your cluster.
# If using k3d to test locally, then probably 10.x.x.x or 192.168.X.X
# When running behind a Service in-cluster; 0.0.0.0
test -n "${ADMISSION_PRIVATE_IP}"

# Cleanup: Remove old config if exists (immutable)
kubectl delete mutatingwebhookconfiguration habitat-admission-controller || true
kubectl delete validatingwebhookconfigurations habitat-admission-controller || true

# If behind a service:
#kubectl -n default delete secret habitat-admission-controller-tls || true

# Create cache dir to save CA certs
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
CACHE_DIR=$SCRIPT_DIR/caches
mkdir -p $CACHE_DIR

# Get your IP into the cert
echo "subjectAltName = IP:${ADMISSION_PRIVATE_IP}" > $CACHE_DIR/admission_extfile.cnf
# Or, if using DNS (e.g. when running behind a service):
#echo "subjectAltName = DNS:habitat-admission-controller.default.svc" > $CACHE_DIR/admission_extfile.cnf

# Generate the CA cert and private key
openssl req -nodes -new -x509 \
    -keyout $CACHE_DIR/ca.key \
    -out $CACHE_DIR/ca.crt -subj "/CN=habitat-admission-controller"

# Generate the private key for the webhook server
openssl genrsa -out $CACHE_DIR/admission-controller-tls.key 2048

# Generate a Certificate Signing Request (CSR) for the private key
# and sign it with the private key of the CA.
openssl req -new -key $CACHE_DIR/admission-controller-tls.key \
    -subj "/CN=habitat-admission-controller" \
    | openssl x509 -req -CA $CACHE_DIR/ca.crt -CAkey $CACHE_DIR/ca.key \
        -CAcreateserial -out $CACHE_DIR/admission-controller-tls.crt \
        -extfile $CACHE_DIR/admission_extfile.cnf

CA_PEM64="$(openssl base64 -A < $CACHE_DIR/admission-controller-tls.crt)"
# shellcheck disable=SC2016
sed -e 's@${CA_PEM_B64}@'"$CA_PEM64"'@g' < admission_controller.yml.tpl |
    sed -e 's@${PRIVATE_IP}@'"$ADMISSION_PRIVATE_IP"'@g'  \
    | kubectl create -f -

# if behind a service:
#kubectl -n default create secret tls habitat-admission-controller-tls \
#    --cert $CACHE_DIR/admission-controller-tls.crt \
#    --key $CACHE_DIR/admission-controller-tls.key
# similar guide: https://www.openpolicyagent.org/docs/v0.11.0/kubernetes-admission-control/

# Sanity:
kubectl get mutatingwebhookconfiguration habitat-admission-controller -oyaml
kubectl get validatingwebhookconfigurations habitat-admission-controller -oyaml
