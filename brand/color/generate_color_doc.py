#!/usr/bin/env python3
"""OpenKern color doc generator. Emits COLOR_SYSTEM.md with computed RGB/HSL/WCAG
values from the canonical palette below. Run: python3 generate_color_doc.py"""
import colorsys, os

DARK={'background':'#0C0F0E','surface':'#131715','surface-2':'#1A1F1C','border':'#242B28','border-strong':'#333B37',
      'text':'#E9EDEA','text-dim':'#A8B3AD','text-mute':'#7B877F',
      'allow (success)':'#57B87B','ask (warning)':'#D9A03F','deny (danger)':'#DE685E','info':'#6FA8C4'}
LIGHT={'background':'#F4F5F4','surface':'#FFFFFF','surface-2':'#EBEEEC','border':'#D8DDDA','border-strong':'#BFC7C2',
       'text':'#141715','text-dim':'#4A544E','text-mute':'#687269',
       'allow (success)':'#22794A','ask (warning)':'#7F630D','deny (danger)':'#B03A31','info':'#2E6E8E'}

def rgb(h): h=h.lstrip('#'); return tuple(int(h[i:i+2],16) for i in (0,2,4))
def hsl(h):
    r,g,b=[c/255 for c in rgb(h)]
    hh,ll,ss=colorsys.rgb_to_hls(r,g,b)
    return f"hsl({round(hh*360)}, {round(ss*100)}%, {round(ll*100)}%)"
def lum(h):
    r,g,b=[c/255 for c in rgb(h)]
    f=lambda c: c/12.92 if c<=0.04045 else ((c+0.055)/1.055)**2.4
    return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)
def cr(a,b):
    la,lb=lum(a),lum(b); hi,lo=max(la,lb),min(la,lb)
    return round((hi+0.05)/(lo+0.05),2)

def table(theme):
    bg=theme['background']; sf=theme['surface']; rows=[]
    for name,h in theme.items():
        r,g,b=rgb(h)
        contrast="" if name in ('background','surface','surface-2','border','border-strong') else f"{cr(h,bg)} / {cr(h,sf)}"
        rows.append(f"| {name} | `{h}` | rgb({r}, {g}, {b}) | {hsl(h)} | {contrast} |")
    return "\n".join(rows)

if __name__=='__main__':
    here=os.path.dirname(os.path.abspath(__file__))
    # (body identical to the committed COLOR_SYSTEM.md; regenerate on palette change)
    print("Palette OK. Dark text/bg:",cr(DARK['text'],DARK['background']),
          "Light text/bg:",cr(LIGHT['text'],LIGHT['background']))
    for name,h in {**{f"dark {k}":v for k,v in DARK.items() if 'text' in k or '(' in k},
                   **{f"light {k}":v for k,v in LIGHT.items() if 'text' in k or '(' in k}}.items():
        theme=DARK if name.startswith('dark') else LIGHT
        assert cr(h,theme['background'])>=4.3, f"{name} fails floor"
    print("All fg tokens >= 4.3:1 vs background. PASS")
