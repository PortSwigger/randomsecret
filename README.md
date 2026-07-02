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
`A_SECRET_WITH_DEFAULT_LENGTH` (44 characters, the default).

## Generated values

Values are base62 strings using the alphabet ordering from
[zqlu](https://github.com/nresare/zqlu) (`0-9`, `A-Z`, `a-z`), with the quirk
that the first character is never a digit. When `length` is omitted, the value
is 44 characters: the smallest length that carries at least 256 bits (32
bytes) of entropy given that the first character is drawn from only the 52
letters.

## Behaviour

- The `Secret` gets the same name and namespace as the `RandomSecret` and an
  owner reference, so deleting the `RandomSecret` garbage-collects the
  `Secret`.
- Existing values are never rotated: reconciliation only generates values for
  keys missing from the `Secret`, and removes keys no longer in the spec.

## Running

```sh
kubectl apply -f manifests/crd.yaml
cargo run          # runs against the current kubeconfig context
```

Regenerate the CRD manifest after changing the spec types:

```sh
cargo run -- crd > manifests/crd.yaml
```

## Trying it out

```sh
kubectl apply -f examples/example.yaml
kubectl get secret my-secrets -o json | jq '.data | map_values(@base64d)'
```

Delete the `RandomSecret` and observe that the `Secret` disappears:

```sh
kubectl delete randomsecret my-secrets
```

## License

MIT
