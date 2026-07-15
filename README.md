# randomsecret

A Kubernetes operator that materializes `RandomSecret` custom resources into
regular `Secret` objects with randomly generated values, so that applications
can declare "these secrets need to exist" without anyone ever handling the
values manually.

```yaml
apiVersion: noa.re/v1alpha1
kind: RandomSecret
metadata:
  name: my-secrets
spec:
  secrets:
    - name: MY_SECRET_NAME
      length: 40
    - name: A_SECRET_WITH_DEFAULT_LENGTH
```

Reconciling the above creates a `Secret` named `my-secrets` in the same
namespace with the keys `MY_SECRET_NAME` (40 characters) and
`A_SECRET_WITH_DEFAULT_LENGTH` (45 characters, the default).

## Generated values

Secrets are made up uppercase and lowercase characters and numbers, with the quirk
that the first character is never a digit. When `length` is omitted, the value
is 45 characters: the smallest length that carries at least 256 bits (32
bytes) of entropy given that the first character is drawn from only the 52
letters (44), rounded up to the next multiple of 3 so that Kubernetes'
base64 encoding of the value needs no padding.

## Behaviour

- The `Secret` gets the same name and namespace as the `RandomSecret` and an
  owner reference, so deleting the `RandomSecret` garbage-collects the
  `Secret`.
- Existing values are never rotated: reconciliation only generates values for
  keys missing from the `Secret`, and removes keys no longer in the spec.

## Deploying with helm

A helm chart installing the CRD, the operator Deployment and its RBAC is
published to Amazon ECR Public at
`oci://public.ecr.aws/portswigger-platform/charts/randomsecret`:

```sh
helm install randomsecret \
  oci://public.ecr.aws/portswigger-platform/charts/randomsecret \
  --namespace randomsecret --create-namespace
```

## License

MIT
