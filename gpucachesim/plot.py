import matplotlib.colors as mc
import colorsys

from gpucachesim.benchmarks import REPO_ROOT_DIR

PLOT_DIR = REPO_ROOT_DIR / "plot"

PPI = 300
FONT_SIZE_PT = 11

DINA4_WIDTH_MM = 210
DINA4_HEIGHT_MM = 297


def mm_to_inch(mm):
    return mm / 25.4


DINA4_WIDTH_INCHES = mm_to_inch(DINA4_WIDTH_MM)
DINA4_HEIGHT_INCHES = mm_to_inch(DINA4_HEIGHT_MM)


def pt_to_px(pt):
    return int(pt * 4.0 / 3.0)


DINA4_WIDTH = PPI * mm_to_inch(DINA4_WIDTH_MM)
DINA4_HEIGHT = PPI * mm_to_inch(DINA4_HEIGHT_MM)
FONT_SIZE_PX = pt_to_px(FONT_SIZE_PT)


def hex_to_rgb(hex_color):
    hex_color = hex_color.lstrip("#")
    if len(hex_color) == 3:
        hex_color = hex_color * 2
    return int(hex_color[0:2], 16), int(hex_color[2:4], 16), int(hex_color[4:6], 16)


def plotly_rgba(r, g, b, a):
    return "rgba(%d, %d, %d, %f)" % (r, g, b, a)


def plt_rgba(r, g, b, a=1.0):
    return (float(r) / 255.0, float(g) / 255.0, float(b) / 255.0, a)


HEX_COLORS = {
    "green1": "#81bc4f",
    "purple1": "#c21b7b",
    "blue1": "#196fac",
}

RGB_COLORS = {k: hex_to_rgb(v) for k, v in HEX_COLORS.items()}

SIM_RGB_COLORS = {
    "gpucachesim": RGB_COLORS["green1"],
    "accelsim": RGB_COLORS["purple1"],
    "native": RGB_COLORS["blue1"],
}

# valid hatches: *+-./OX\ox|
SIM_HATCHES = {
    "gpucachesim": "/",
    "accelsim": "+",
    "native": "x",
}


def plt_lighten_color(color, amount=0.5):
    """
    Lightens the given color by multiplying (1-luminosity) by the given amount.
    Input can be matplotlib color string, hex string, or RGB tuple.

    Examples:
    >> lighten_color('g', 0.3)
    >> lighten_color('#F034A3', 0.6)
    >> lighten_color((.3,.55,.1), 0.5)
    """
    try:
        c = mc.cnames[color]
    except:
        c = color
    c = colorsys.rgb_to_hls(*mc.to_rgb(c))
    return colorsys.hls_to_rgb(c[0], 1 - amount * (1 - c[1]), c[2])


def plt_darken_color(color, amount=0.5):
    return plt_lighten_color(color, 1.0 + amount)


def human_format_thousands(num, round_to=2):
    magnitude = 0
    while abs(num) >= 1000:
        magnitude += 1
        num = round(num / 1000.0, round_to)
    return "{:.{}f}{}".format(num, round_to, ["", "K", "M", "G", "T", "P"][magnitude])
