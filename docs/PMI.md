# PMI / drawing alpha (H2-8)

**Not a drafting package.** Dimension facts attached to selectors → JSON drawing packet.
No sheets, title blocks, GD&T symbols, or STEP AP242 PMI.

## CLI

```sh
# Auto: opposite-face linear dims
cargo run -p cadrion-cli -- inspect dims examples/pmi/block.cad.star --json

# Explicit
cargo run -p cadrion-cli -- inspect dims examples/pmi/block.cad.star \
  --dim '#o1.1.f1,#o1.1.f2,thickness,height' --json

# Specs file
cargo run -p cadrion-cli -- inspect dims examples/pmi/block.cad.star \
  --specs examples/pmi/block.dims.json -o /tmp/block.drawing.json --json
```

Default output: `<stem>.drawing.json` next to the source.

## Packet schema (`cadrion.drawing_packet` v1)

```json
{
  "ok": true,
  "schema": "cadrion.drawing_packet",
  "version": 1,
  "source": "block.cad.star",
  "topology": "ir-analytic",
  "dims": [
    {
      "id": "d1",
      "kind": "linear",
      "a": "#o1…",
      "b": "#o1…",
      "value": 20.0,
      "unit": "mm",
      "construction": "…",
      "label": "auto-opposite"
    }
  ],
  "notes": ["PMI alpha …"]
}
```

## Kinds

| kind | needs | unit |
|------|-------|------|
| `distance` / `linear` | A,B | mm |
| `thickness` | A,B faces | mm |
| `diameter` | A face/edge | mm |
| `angle` | A,B faces | deg |

## Viewer overlay (H3-5)

`cadrion view part.cad.star` embeds dim chips on the orbit canvas from `drawing.json`
(auto or sidecar). See `docs/VIEWER.md`. Still not a drafting package.
