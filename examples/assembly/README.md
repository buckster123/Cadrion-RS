# Assembly example (S9)

Plate + catalog bolt with `parts.lock`.

```sh
# from repo root
cargo run -p cadrion-cli -- serve api --project examples/assembly --port 7410 --token dev

curl -s -H "Authorization: Bearer dev" -H 'content-type: application/json' \
  -d '{"path":"plate_bolt.assy.json"}' \
  http://127.0.0.1:7410/v1/assembly/validate

curl -s -H "Authorization: Bearer dev" -H 'content-type: application/json' \
  -d '{"path":"cad/plate.cad.star"}' \
  http://127.0.0.1:7410/v1/build
```

`parts/m6_bolt.step` is a **stub** STEP text for lock/provider tests (not real geometry).
