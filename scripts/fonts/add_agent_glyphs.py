"""Add the bundled agent marks to the existing Herdr font.

Run with: uv run --with fonttools python scripts/fonts/add_agent_glyphs.py
"""
from pathlib import Path
import xml.etree.ElementTree as ET

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import parse_path
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]
FONT = ROOT / 'assets/fonts/HerdrAgentIconsMax-Regular.ttf'
MARKS = ROOT / 'assets/fonts/marks'
GLYPHS = {
    'minimax': 0xE1BC, 'xiaomimimo': 0xE1BD, 'goose': 0xE1BE,
    'openhands': 0xE1BF, 'codebuddy': 0xE1C6, 'trae': 0xE1C7,
    'junie': 0xE1C8, 'zedindustries': 0xE1C9, 'warp': 0xE1CA,
}


def main():
    font = TTFont(FONT)
    cmap = font.getBestCmap()
    advance = font['hmtx'][cmap[0xE1A1]][0]
    for name, codepoint in GLYPHS.items():
        glyph_name = 'recall_' + name
        assert codepoint not in cmap or cmap[codepoint] == glyph_name, hex(codepoint)
        root = ET.parse(MARKS / (name + '.svg')).getroot()
        paths = list(root.iter('{http://www.w3.org/2000/svg}path'))
        assert paths and all(not p.get('transform') for p in paths), name
        recording = RecordingPen()
        for path in paths:
            parse_path(path.attrib['d'], recording)
        bounds_pen = BoundsPen(None)
        recording.replay(bounds_pen)
        x0, y0, x1, y1 = bounds_pen.bounds
        scale = 918 / max(x1 - x0, y1 - y0)
        target = TTGlyphPen(None)
        quad = Cu2QuPen(target, max_err=1, reverse_direction=True)
        transform = TransformPen(quad, (scale, 0, 0, -scale,
                                        advance / 2 - scale * (x0 + x1) / 2,
                                        365 + scale * (y0 + y1) / 2))
        recording.replay(transform)
        glyph = target.glyph()
        font['glyf'][glyph_name] = glyph
        if glyph_name not in font.getGlyphOrder():
            font.setGlyphOrder(font.getGlyphOrder() + [glyph_name])
        font['hmtx'][glyph_name] = (advance, 0)
        for table in font['cmap'].tables:
            if table.isUnicode():
                table.cmap[codepoint] = glyph_name
    for record in font['name'].names:
        if record.nameID == 5:
            version = record.toUnicode().split('; Recall patch')[0] + '; Recall patch'
            record.string = version.encode(record.getEncoding())
    font.save(FONT)


if __name__ == '__main__':
    main()
