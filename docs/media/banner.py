"""The README's banner (docs/media/banner.webp): every creature of the series, done or planned, at sunrise.

Drawn from the model pictures (docs/models/<mod>-<model>-front.png, `cargo creature preview`): a new model goes in
`items` below (its picture, its height in pixels, its x, its feet's y, mirrored or not), then, from the repository's
root, `python3 docs/media/banner.py` (Pillow). One model per mod (one flower for the Living Flowers).
"""
from PIL import Image, ImageDraw, ImageFilter, ImageOps
import random, sys
W,H=1600,520
bg=Image.new('RGB',(W,H)); d=ImageDraw.Draw(bg)
top=(46,32,84); mid=(214,96,84); low=(250,184,110)
for y in range(H):
    t=y/(H*0.78)
    if t<0.55:
        k=t/0.55; c=tuple(int(top[i]+(mid[i]-top[i])*k) for i in range(3))
    else:
        k=min(1,(t-0.55)/0.45); c=tuple(int(mid[i]+(low[i]-mid[i])*k) for i in range(3))
    d.line([(0,y),(W,y)],fill=c)
glow=Image.new('L',(W,H),0); gd=ImageDraw.Draw(glow)
cx,cy=W//2,int(H*0.80)
for r in range(260,0,-4):
    gd.ellipse([cx-r,cy-r,cx+r,cy+r],fill=int(150*(1-r/260)**1.6))
bg=Image.composite(Image.new('RGB',(W,H),(255,226,160)),bg,glow.filter(ImageFilter.GaussianBlur(8)))
d=ImageDraw.Draw(bg); d.ellipse([cx-70,cy-70,cx+70,cy+70],fill=(255,236,178))
def hills(base,amp,col,step,seed):
    random.seed(seed); y=base; poly=[(0,H)]
    for x in range(0,W+step,step):
        y=max(base-amp,min(base+amp,y+random.choice([-step//2,0,step//2]))); poly+=[(x,y),(x+step,y)]
    poly.append((W,H)); d.polygon(poly,fill=col)
hills(int(H*0.80),40,(120,70,92),48,4); hills(int(H*0.88),24,(58,52,70),32,9)
d.rectangle([0,int(H*0.94),W,H],fill=(40,38,52))
bg=bg.convert('RGBA')
def model(name,h,flip=False):
    im=Image.open(f'docs/models/{name}-front.png').convert('RGBA'); im=im.crop(im.getbbox())
    if flip: im=ImageOps.mirror(im)
    s=h/im.height; return im.resize((max(1,int(im.width*s)),int(h)),Image.NEAREST)
G=int(H*0.93)
# (picture, height, x centre, feet y, mirrored); creatures on the ground first (shadowed), then the flyers.
items=[
 ('magma_golem-magma_golem',290,170,G,False),
 ('dung_beetles-dung_beetle',58,345,G,False),
 ('worms-worm',34,415,G,False),
 ('sporelins-sporelin',88,480,G,False),
 ('living_flowers-poppy',96,555,G,False),
 ('cyclops-cyclops',300,690,G,False),
 ('vulture-vulture',135,995,G,False),
 ('scorpions-scorpion',62,1215,G,False),
 ('phasmids-stick_insect',46,1305,G,False),
 ('fairies-fairy',140,1420,G-36,False),
 ('seagulls-seagull',105,420,G-330,True),
 ('flying_fish-flying_fish',62,545,G-205,False),
 ('jellyfish-jellyfish',105,1000,G-335,False),
 ('fireflies-firefly',42,1075,G-245,False),
 ('butterflies-butterfly',85,1265,G-205,False),
 ('butterflies-moth',50,1345,G-330,False),
 ('storm_spirits-storm_spirit',130,1520,G-330,False),
]
for name,h,x,y,flip in items:
    m=model(name,h,flip)
    if y==G:
        sh=Image.new('RGBA',bg.size,(0,0,0,0)); ImageDraw.Draw(sh).ellipse([x-m.width*0.4,y-8,x+m.width*0.4,y+8],fill=(0,0,0,90))
        bg=Image.alpha_composite(bg,sh.filter(ImageFilter.GaussianBlur(4)))
    bg.alpha_composite(m,(int(x-m.width/2),int(y-m.height)))
bg.convert('RGB').save('docs/media/banner.webp',quality=88)
if len(sys.argv) > 1:
    bg.convert('RGB').resize((1000,325)).save(sys.argv[1])  # a small copy to look at
