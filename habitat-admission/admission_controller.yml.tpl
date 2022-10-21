---
apiVersion: admissionregistration.k8s.io/v1
kind: MutatingWebhookConfiguration
metadata:
  name: habitat-admission-controller
webhooks:
  - name: admission-controller.habitat.svc
    # Optionally restrict events from namespaces with a specific label.
    # namespaceSelector:
    #   matchLabels:
    #     some-label: "true"
    clientConfig:
      caBundle: "${CA_PEM_B64}"
      url: "https://${PRIVATE_IP}:8443/mutate"
      # For controllers behind k8s services, use the format below instead of a url
      #service:
      #  name: foo-admission
      #  namespace: default
      #  path: "/mutate"
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
    # Optionally restrict events from namespaces with a specific label.
    # namespaceSelector:
    #   matchLabels:
    #     some-label: "true"
    clientConfig:
      caBundle: "${CA_PEM_B64}"
      url: "https://${PRIVATE_IP}:8443/validate"
      # For controllers behind k8s services, use the format below instead of a url
      #service:
      #  name: foo-admission
      #  namespace: default
      #  path: "/validate"
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["batch.habitat"]
        apiVersions: ["v1beta1"]
        resources: ["jobs"]
    failurePolicy: Fail
    admissionReviewVersions: ["v1", "v1beta1"]
    sideEffects: None
    timeoutSeconds: 5