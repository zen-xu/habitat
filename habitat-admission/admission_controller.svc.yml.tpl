---
apiVersion: admissionregistration.k8s.io/v1
kind: MutatingWebhookConfiguration
metadata:
  name: habitat-admission-controller
webhooks:
  - name: admission-controller.habitat.svc
    clientConfig:
      service:
        name: habitat-admission-controller-tls
        namespace: ${NAMESPACE}
        path: "/mutate"
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["batch.habitat"]
        apiVersions: ["v1beta1"]
        resources: ["jobs"]
    failurePolicy: Fail
    admissionReviewVersions: ["v1", "v1beta1"]
    sideEffects: None
    timeoutSeconds: 5
---
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: habitat-admission-controller
webhooks:
  - name: admission-controller.habitat.svc
    clientConfig:
      service:
        name: habitat-admission-controller-tls
        namespace: ${NAMESPACE}
        path: "/validate"
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["batch.habitat"]
        apiVersions: ["v1beta1"]
        resources: ["jobs"]
    failurePolicy: Fail
    admissionReviewVersions: ["v1", "v1beta1"]
    sideEffects: None
    timeoutSeconds: 5