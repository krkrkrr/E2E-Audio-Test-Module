"""doc/ の検査。文書が実装やほかの文書とずれる典型的な壊れ方を機械的に見つける。"""

import re
from pathlib import Path

import pytest

DOC = Path(__file__).parents[2] / "doc"
MARKDOWN = sorted(DOC.rglob("*.md"))
RESERVED = {"index.md", "log.md"}

LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
FENCE = re.compile(r"^```.*?^```", re.S | re.M)


def rel(p: Path) -> str:
    return str(p.relative_to(DOC))


def frontmatter(text: str) -> dict[str, str]:
    if not text.startswith("---\n"):
        return {}
    block = text[4 : text.index("\n---", 4)]
    return dict(line.split(":", 1) for line in block.splitlines() if ":" in line and not line.startswith(" "))


@pytest.mark.parametrize("path", [p for p in MARKDOWN if p.name not in RESERVED], ids=rel)
def test_every_markdown_has_an_okf_type(path):
    fm = frontmatter(path.read_text(encoding="utf-8"))
    assert fm.get("type", "").strip(), f"{rel(path)} has no OKF type"


@pytest.mark.parametrize("path", MARKDOWN, ids=rel)
def test_relative_links_point_at_existing_files(path):
    text = FENCE.sub("", path.read_text(encoding="utf-8"))
    for target in LINK.findall(text):
        if re.match(r"^[a-z]+:", target) or target.startswith("#"):
            continue
        resolved = (path.parent / target.split("#")[0]).resolve()
        assert resolved.exists(), f"{rel(path)}: broken link {target}"


@pytest.mark.parametrize("path", MARKDOWN, ids=rel)
def test_mermaid_diagrams_stay_within_40_lines_and_7_classes(path):
    for block in re.findall(r"^```mermaid\n(.*?)^```", path.read_text(encoding="utf-8"), re.S | re.M):
        lines = block.splitlines()
        classes = [l for l in lines if re.match(r"\s*class \w+ \{", l)]
        assert len(lines) <= 40, f"{rel(path)}: diagram has {len(lines)} lines"
        assert len(classes) <= 7, f"{rel(path)}: diagram has {len(classes)} classes"


def test_use_case_ids_carry_a_context_prefix():
    text = (DOC / "use-cases.md").read_text(encoding="utf-8")
    ids = re.findall(r"^\| ([A-Z]+-\d+) \|", text, re.M)
    assert ids, "no use cases found"
    assert all(re.fullmatch(r"(AUD|CAP)-\d+", i) for i in ids), ids
    assert len(ids) == len(set(ids)), "duplicate use case id"
    assert not re.search(r"\bUC-\d", "\n".join(p.read_text(encoding="utf-8") for p in MARKDOWN))


HEADINGS = {
    "domain-model.md": ["## この文脈の言語", "## 集約と値オブジェクト", "## 対応する src"],
    "object-model.md": ["## この文脈が担うユースケース", "## やりとり"],
    "constraints.md": ["## 守るべき規則", "## 不変条件"],
}


@pytest.mark.parametrize("context", sorted(p.name for p in (DOC / "context").iterdir()))
def test_context_documents_use_the_shared_headings(context):
    for name, headings in HEADINGS.items():
        text = (DOC / "context" / context / name).read_text(encoding="utf-8")
        for h in headings:
            assert re.search(rf"^{re.escape(h)}$", text, re.M), f"{context}/{name} lacks {h}"
        assert "system-context.md" in text, f"{context}/{name} does not link system-context.md"
