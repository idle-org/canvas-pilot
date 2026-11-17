#!/usr/bin/env python3
"""Build an offline Lumen Forum recording kit; requires rsvg-convert and ffmpeg.

The speaker is a supplied synthetic PNG; no network, device, or image-generation
calls are made by this recipe. Native PPTX packages reuse the repository's starter
master/theme and contain editable shapes/text or independently movable pictures.
"""
import argparse
import base64
import copy
import csv
import html
import json
from pathlib import Path
import shutil
import struct
import subprocess
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]
W, H = 1920, 1080
NAVY, PANEL, TEAL, GOLD, WHITE, MUTED = '#071b2b', '#102e40', '#56d9ca', '#edbd73', '#f5f1e8', '#a5b9c4'
P, A, R = ('http://schemas.openxmlformats.org/' + x for x in (
    'presentationml/2006/main', 'drawingml/2006/main', 'officeDocument/2006/relationships'))
REL = 'http://schemas.openxmlformats.org/package/2006/relationships'
CT = 'http://schemas.openxmlformats.org/package/2006/content-types'
LAYOUTS = {
    '01-speaker-and-slides': [('Speaker', 72, 260, 432, 576), ('Presentation', 552, 260, 1296, 729)],
    '02-slide-focus': [('Presentation', 96, 54, 1728, 972)],
    '03-side-by-side': [('Speaker', 72, 288, 864, 486), ('Presentation', 984, 288, 864, 486)],
}


def run(args):
    subprocess.run([str(a) for a in args], check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.PIPE, timeout=180)


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data) if isinstance(data, bytes) else path.write_text(data, encoding='utf-8')


def png_dimensions(path):
    with path.open('rb') as source:
        header = source.read(24)
    if len(header) != 24 or header[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('Speaker must be a PNG')
    width, height = struct.unpack('>II', header[16:24])
    if not width or not height or width * 4 != height * 3:
        raise ValueError('Speaker must have a 3:4 aspect ratio to avoid stretching')
    return width, height


def rect(x, y, w, h, fill, name='Decor', stroke=None, kind='rect', role='background'):
    return dict(kind=kind, x=x, y=y, w=w, h=h, fill=fill, name=name, stroke=stroke, role=role)


def text(x, y, value, size=28, color=WHITE, bold=False, width=1700):
    return dict(kind='text', x=x, y=y, w=width, h=size*1.35, text=value, size=size,
                fill=color, bold=bold, name=value, role='background')


def picture(x, y, w, h, path, name, role='layer'):
    return dict(kind='image', x=x, y=y, w=w, h=h, path=path, name=name, role=role)


def svg(scene, marked=False):
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{W}" height="{H}" viewBox="0 0 {W} {H}"' + (' data-midra-layout-geometry="2"' if marked else '') + '>']
    for i, e in enumerate(scene):
        x,y,w,h = [e[k] for k in ('x','y','w','h')]
        attrs = f'id="shape-{i}" data-midra-role="{e["role"]}"'
        title = f'<title>{html.escape(e["name"])}</title>'
        if e['kind'] == 'image':
            data = base64.b64encode(e['path'].read_bytes()).decode()
            parts.append(f'<image {attrs} x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="xMidYMid slice" xlink:href="data:image/png;base64,{data}">{title}</image>')
        elif e['kind'] == 'text':
            parts.append(f'<text x="{x}" y="{y+e["size"]}" font-family="DejaVu Sans" font-size="{e["size"]}" font-weight="{700 if e["bold"] else 400}" fill="{e["fill"]}">{html.escape(e["text"])}</text>')
        else:
            style = f'fill="{e["fill"] or "none"}" stroke="{e["stroke"] or "none"}" stroke-width="2"'
            if e['kind'] == 'ellipse':
                parts.append(f'<ellipse cx="{x+w/2}" cy="{y+h/2}" rx="{w/2}" ry="{h/2}" {style}>{title}</ellipse>')
            else:
                parts.append(f'<rect {attrs} x="{x}" y="{y}" width="{w}" height="{h}" {style}>{title}</rect>')
    return '\n'.join(parts+['</svg>'])


def render(scene, stem, jpeg=True):
    save(stem.with_suffix('.svg'), svg(scene))
    run(['rsvg-convert', '-o', stem.with_suffix('.png'), stem.with_suffix('.svg')])
    if jpeg:
        run(['ffmpeg','-v','error','-y','-i',stem.with_suffix('.png'),'-frames:v','1','-q:v','2',stem.with_suffix('.jpg')])


def drawio(scene, path):
    root = ET.Element('mxfile', host='Canvas Pilot showcase', compressed='false')
    diagram = ET.SubElement(root,'diagram',id='lumen',name='Lumen Forum')
    model = ET.SubElement(diagram,'mxGraphModel',page='1',pageScale='1',pageWidth=str(W),pageHeight=str(H),grid='0',adaptiveColors='none', **{'data-midra-layout-geometry':'1'})
    cells = ET.SubElement(model,'root')
    ET.SubElement(cells,'mxCell',id='0'); ET.SubElement(cells,'mxCell',id='1',parent='0')
    for i,e in enumerate(scene):
        style = 'rounded=0;whiteSpace=wrap;html=0;strokeColor=none;'
        if e['kind'] == 'image':
            style += 'shape=image;imageAspect=0;image=data:image/png,' + base64.b64encode(e['path'].read_bytes()).decode() + ';'
        else:
            style += f'fillColor={e["fill"]};fontColor=#ffffff;'
        cell = ET.SubElement(cells,'mxCell',id=f'layer-{i+1}',value=e['name'] if e['role']=='layer' else '',vertex='1',parent='1',style=style, **{'data-midra-role':e['role']})
        ET.SubElement(cell,'mxGeometry',x=str(e['x']),y=str(e['y']),width=str(e['w']),height=str(e['h']), **{'as':'geometry'})
    save(path,ET.tostring(root,encoding='utf-8',xml_declaration=True))


def pptx(scenes, path):
    """Native text, rectangles and pictures, in 96-pixel/inch slide coordinates."""
    for prefix, uri in [('p', P), ('a', A), ('r', R)]:
        ET.register_namespace(prefix, uri)
    with zipfile.ZipFile(ROOT/'docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.pptx') as z:
        package = {n:z.read(n) for n in z.namelist()}
    pres = ET.fromstring(package['ppt/presentation.xml'])
    ids = pres.find(f'{{{P}}}sldIdLst'); ids.clear()
    rels = ET.fromstring(package['ppt/_rels/presentation.xml.rels'])
    for rel in list(rels):
        if rel.attrib['Type'].endswith('/slide'): rels.remove(rel)
    types = ET.fromstring(package['[Content_Types].xml'])
    for entry in list(types):
        if entry.attrib.get('PartName','').startswith('/ppt/slides/'): types.remove(entry)
    ET.SubElement(types,f'{{{CT}}}Default',Extension='png',ContentType='image/png')
    for number,scene in enumerate(scenes,1):
        ET.SubElement(ids,f'{{{P}}}sldId',id=str(255+number), **{f'{{{R}}}id':f'showcase{number}'})
        ET.SubElement(rels,f'{{{REL}}}Relationship',Id=f'showcase{number}',Type=R+'/slide',Target=f'slides/slide{number}.xml')
        ET.SubElement(types,f'{{{CT}}}Override',PartName=f'/ppt/slides/slide{number}.xml',ContentType='application/vnd.openxmlformats-officedocument.presentationml.slide+xml')
        shapes=[]; image_rels=[]
        for i,e in enumerate(scene,2):
            x,y,w,h=[round(e[k]*9525) for k in ('x','y','w','h')]
            transform=f'<a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm>'
            name=html.escape('midra-background' if e['kind']=='image' and e['role']=='background' else e['name'],quote=True)
            if e['kind']=='image':
                media=f'showcase-{number}-{i}.png'; package['ppt/media/'+media]=e['path'].read_bytes()
                image_rels.append(f'<Relationship Id="img{i}" Type="{R}/image" Target="../media/{media}"/>')
                shapes.append(f'<p:pic><p:nvPicPr><p:cNvPr id="{i}" name="{name}"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="img{i}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{transform}<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>')
                continue
            color=(e['fill'] or '#000000').lstrip('#')
            fill=f'<a:solidFill><a:srgbClr val="{color}"/></a:solidFill>' if e['fill'] and e['kind']!='text' else '<a:noFill/>'
            stroke=f'<a:ln w="19050"><a:solidFill><a:srgbClr val="{e["stroke"].lstrip("#")}"/></a:solidFill></a:ln>' if e.get('stroke') else '<a:ln><a:noFill/></a:ln>'
            geom='ellipse' if e['kind']=='ellipse' else 'rect'
            body=''
            if e['kind']=='text':
                size=round(e['size']*75)
                body=f'<p:txBody><a:bodyPr wrap="none" lIns="0" rIns="0" tIns="0" bIns="0" anchor="t"/><a:lstStyle/><a:p><a:pPr><a:lnSpc><a:spcPct val="100000"/></a:lnSpc></a:pPr><a:r><a:rPr lang="en-US" sz="{size}" b="{int(e["bold"])}"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:latin typeface="DejaVu Sans"/></a:rPr><a:t>{html.escape(e["text"])}</a:t></a:r><a:endParaRPr sz="{size}"/></a:p></p:txBody>'
            shapes.append(f'<p:sp><p:nvSpPr><p:cNvPr id="{i}" name="{name}"/><p:cNvSpPr txBox="{int(e["kind"]=="text")}"/><p:nvPr/></p:nvSpPr><p:spPr>{transform}<a:prstGeom prst="{geom}"><a:avLst/></a:prstGeom>{fill}{stroke}</p:spPr>{body}</p:sp>')
        content=f'<p:sld xmlns:p="{P}" xmlns:a="{A}" xmlns:r="{R}" showMasterSp="0"><p:cSld name="Lumen {number}"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>{"".join(shapes)}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>'
        package[f'ppt/slides/slide{number}.xml']=content.encode()
        package[f'ppt/slides/_rels/slide{number}.xml.rels']=(f'<Relationships xmlns="{REL}"><Relationship Id="rId1" Type="{R}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>{"".join(image_rels)}</Relationships>').encode()
    for n,root in [('ppt/presentation.xml',pres),('ppt/_rels/presentation.xml.rels',rels),('[Content_Types].xml',types)]:
        if n.endswith('.rels'): ET.register_namespace('', REL)
        if n == '[Content_Types].xml': ET.register_namespace('', CT)
        package[n]=ET.tostring(root,encoding='utf-8',xml_declaration=True)
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,data in package.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
            z.writestr(info,data)


def background():
    return [rect(0,0,W,H,NAVY),rect(1450,0,470,H,'#0b2637'),
        rect(1530,100,650,650,None,stroke='#194456',kind='ellipse'),
        rect(1620,190,470,470,None,stroke='#194456',kind='ellipse'),
        rect(72,64,14,34,TEAL),text(104,62,'LUMEN / FORUM',28,bold=True),
        text(72,115,'Ideas into impact.',66,bold=True),
        text(75,202,'PEOPLE  /  DESIGN  /  POSITIVE CHANGE',22,MUTED),
        text(1550,64,'THE NEXT CHAPTER',18,GOLD,width=300),
        text(1550,96,'STUDIO 01  •  2026',20,width=300),
        rect(72,1014,1776,2,'#31515e'),text(72,1034,'LUMEN FORUM',17,GOLD),
        text(660,1034,'MAKE SPACE FOR WHAT COMES NEXT.',17,MUTED),
        text(1610,1034,'FICTIONAL DEMO',17,MUTED,width=240)]


def frame_scene():
    scene=background()
    scene += [text(72,228,'01 / ON STAGE',18,TEAL),text(552,228,'02 / THE BIG PICTURE',18,TEAL)]
    for name,x,y,w,h in LAYOUTS['01-speaker-and-slides']:
        scene += [rect(x-3,y-3,w+6,h+6,'#315566'),rect(x,y,w,h,PANEL)]
    scene += [rect(72,861,42,4,GOLD),text(72,882,'Alex Morgan',32,bold=True,width=460),
        text(72,928,'Design researcher',22,MUTED,width=460),text(72,962,'A fictional keynote',18,MUTED,width=460)]
    return scene


def slides():
    def base(n,section):
        return [rect(0,0,W,H,'#f5f1e8'),rect(0,0,24,H,TEAL),text(96,70,'LUMEN / FORUM',24,NAVY,True),
            text(1340,70,section,22,'#536d72',width=500),rect(96,964,1728,2,'#ced6ce'),
            text(96,994,'ILLUSTRATIVE DEMO DATA · FICTIONAL EVENT',18,'#536d72'),text(1740,992,f'0{n}',24,NAVY,True,width=80)]
    one=base(1,'01 / FROM SIGNAL TO ACTION')
    one += [text(96,210,'From signals',82,NAVY,True),text(96,310,'to useful change.',82,NAVY,True),
        text(100,464,'Start with people.',34,'#476168'),text(100,518,'Test something small.',34,'#476168'),
        text(100,572,'Build on what works.',34,'#476168'),rect(96,720,690,132,NAVY),
        text(128,745,'Progress is a practice.',36,WHITE,True,width=640),text(128,800,'Not a single big reveal.',24,TEAL,width=640)]
    for x,h,value,label,c in [(1010,240,'42','SIGNALS','#96b8b4'),(1280,390,'68','TRIALS','#329f99'),(1550,540,'86','ADOPTION',NAVY)]:
        one += [rect(x,840-h,180,h,c),text(x,770-h,value,48,NAVY,True,width=190),text(x,870,label,20,'#476168',True,width=230)]
    two=base(2,'02 / A SIMPLE WORKING MODEL')
    two += [text(96,180,'Three moves. One direction.',72,NAVY,True)]
    for x,num,title,lines in [(96,'01','Listen',['Find the friction.','Ask better questions.']),(684,'02','Prototype',['Make the idea visible.','Keep the first test small.']),(1272,'03','Learn',['Share the evidence.','Choose the next step.'])]:
        two += [rect(x,374,552,466,'#e5e9df'),rect(x,374,552,8,TEAL),text(x+36,416,num,64,'#299d96',True,width=480),text(x+36,548,title,44,NAVY,True,width=480),text(x+36,644,lines[0],27,'#476168',width=480),text(x+36,692,lines[1],27,'#476168',width=480)]
    three=base(3,'03 / KEEP THE MOMENTUM')
    three += [text(96,184,'Small pilots. Lasting impact.',72,NAVY,True),
        rect(96,362,810,392,NAVY),text(140,402,'72%',140,TEAL,True,width=700),text(144,600,'would try the next pilot',34,WHITE,width=700),
        rect(954,362,870,392,'#e5e9df'),text(1000,402,'3 weeks',112,NAVY,True,width=780),text(1004,600,'from question to first learning',32,'#476168',width=790),
        text(100,818,'What is the smallest useful thing we can test next?',40,NAVY,True)]
    return [one,two,three]


def geometry_files(out, name, boxes):
    directory=out/'layouts'/name;directory.mkdir(parents=True)
    scene=[rect(x,y,w,h,'#24546a',label,role='layer') for label,x,y,w,h in boxes]
    save(directory/'layout.svg',svg(scene,marked=True));drawio(scene,directory/'layout.drawio');pptx([scene],directory/'layout.pptx')
    layers=[dict(id=f'layer-{i+1}',name=label,role='layer',x=x,y=y,width=w,height=h) for i,(label,x,y,w,h) in enumerate(boxes)]
    save(directory/'layout.json',json.dumps(dict(format='beffroi.midra-geometry',version=1,units='px',canvas=dict(width=W,height=H),layers=layers),indent=2)+'\n')
    with (directory/'layout.csv').open('w',newline='') as f:
        writer=csv.writer(f);writer.writerow(['midra_geometry_version','canvas_width','canvas_height','id','name','role','x','y','width','height'])
        writer.writerows((1,W,H,f'layer-{i+1}',label,'layer',x,y,w,h) for i,(label,x,y,w,h) in enumerate(boxes))
    project=json.loads((ROOT/'docs/interfaces/canvas-pilot-v18.example.json').read_text())
    speaker_size=png_dimensions(out/'media/speaker/alex-morgan-synthetic.png')
    template=copy.deepcopy(project['layers'][0]);project['layers']=[];project['inputs']=[]
    for i,(label,x,y,w,h) in enumerate(boxes,1):
        layer=copy.deepcopy(template);layer.update(id=i,source_id=i,bounds=dict(x=x,y=y,width=w,height=h),name=label,folder='',device_link=None,layer_fit='centered' if label=='Speaker' else 'fullscreen',visible=True,locked=False)
        layer['advanced_crop']=dict.fromkeys(['left','right','top','bottom'],0.0)
        layer['mask']=dict(origin='layer',left=0.0,right=0.0,top=0.0,bottom=0.0)
        project['layers'].append(layer)
        project['inputs'].append(dict(id=i,source_format='custom',native_resolution=dict(width=speaker_size[0] if label=='Speaker' else W,height=speaker_size[1] if label=='Speaker' else H),input_crop=dict.fromkeys(['left','right','top','bottom'],0.0)))
    project.update(next_layer_id=len(boxes)+1,folders=[],input_proposals=[])
    project['editor'].update(backdrop_title='Lumen Forum',selected_layer_ids=[1],position_point='top_left',position_layer_point=None,viewport_zoom=1,viewport_pan=[0,0],outline_layers=True,show_source_pattern=False)
    project['workspace'].update(previews=[],collapsed_folders=[])
    project['workspace']['snapping'].update(show_grid=False,enabled=True)
    reference=out/'delivery-jpg-only'/'program-example.png'
    project['references']=[dict(name=reference.name,image_base64=base64.b64encode(reference.read_bytes()).decode(),visible=name=='01-speaker-and-slides',opacity=1.0,preserve_ratio=True,native_size=True,offset=[0.0,0.0])]
    save(directory/'ready-to-align.pilot.json',json.dumps(project,indent=2)+'\n')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--speaker',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'showcase-conference-kit')
    args=parser.parse_args();out=args.output.resolve()
    if out.exists(): raise SystemExit(f'Refusing to replace existing kit: {out}')
    if not args.speaker.is_file(): raise SystemExit('Speaker PNG does not exist')
    png_dimensions(args.speaker)
    for tool in ['rsvg-convert','ffmpeg','ffprobe']:
        if not shutil.which(tool): raise SystemExit(f'Required rendering tool missing: {tool}')
    for folder in ['media/speaker','media/slides','media/backgrounds','media/video','delivery-jpg-only','delivery-editable','layouts','sources','verification']:
        (out/folder).mkdir(parents=True,exist_ok=True)
    speaker=out/'media/speaker/alex-morgan-synthetic.png';shutil.copyfile(args.speaker,speaker)
    prompt=ROOT/'scripts/showcase-speaker-prompt.txt'
    shutil.copyfile(prompt,out/'sources/speaker-generation-prompt.txt')
    save(out/'sources/provenance.json',json.dumps(dict(event='Lumen Forum — fictional demo',speaker='Alex Morgan — fictional AI-generated person',method='Built-in image_gen tool',prompt='speaker-generation-prompt.txt',data='All slide metrics are invented demonstration data',artwork='Original code-native layout, text, shapes and charts',video='Silent synthetic demo loops; speaker animation is a still-image pan/zoom, not a recording of a live talk'),indent=2)+'\n')
    render(background(),out/'media/backgrounds/background-only')
    render(frame_scene(),out/'media/backgrounds/infodecor-empty')
    guide=frame_scene()+[text(112,460,'SPEAKER',40,TEAL,True,width=360),text(112,528,'432 × 576  /  3:4',24,width=360),text(112,570,'X 72   Y 260',22,MUTED,width=360),text(780,530,'PRESENTATION',64,TEAL,True,width=1000),text(780,630,'1296 × 729  /  16:9',36,width=1000),text(780,690,'X 552   Y 260',28,MUTED,width=1000)]
    render(guide,out/'media/backgrounds/infodecor-placement-guide')
    pptx([background(),frame_scene()],out/'delivery-editable/background-and-infodecor.pptx')
    deck=slides()
    for i,scene in enumerate(deck,1): render(scene,out/f'media/slides/slide-{i:02d}')
    pptx(deck,out/'delivery-editable/presentation.pptx')
    # A native editable rectangle/text rendition of the slides is supplied as SVG.
    for i in range(1,4): shutil.copyfile(out/f'media/slides/slide-{i:02d}.svg',out/f'delivery-editable/slide-{i:02d}.svg')
    shutil.copyfile(out/'media/backgrounds/background-only.svg',out/'delivery-editable/background-only.svg')
    shutil.copyfile(out/'media/backgrounds/infodecor-empty.svg',out/'delivery-editable/infodecor-empty.svg')
    # Pictures have exact source/destination ratios; no hidden stretching/cropping.
    composite=[picture(0,0,W,H,out/'media/backgrounds/infodecor-empty.png','Infodecor','background'),picture(72,260,432,576,speaker,'Speaker'),picture(552,260,1296,729,out/'media/slides/slide-01.png','Presentation')]
    render(composite,out/'delivery-jpg-only/program-example')
    drawio(composite,out/'delivery-editable/composition.drawio');pptx([composite],out/'delivery-editable/composition.pptx')
    save(out/'delivery-editable/composition.svg',svg(composite,marked=True))
    for name in ['background-only','infodecor-empty']:
        for ext in ['png','jpg']: shutil.copyfile(out/f'media/backgrounds/{name}.{ext}',out/f'delivery-jpg-only/{name}.{ext}')
    for ext in ['png','jpg']: shutil.copyfile(out/f'media/slides/slide-01.{ext}',out/f'delivery-jpg-only/presentation-slide.{ext}')
    for name,boxes in LAYOUTS.items(): geometry_files(out,name,boxes)
    save(out/'sources/geometry-truth.json',json.dumps(dict(canvas=[W,H],units='pixels; origin top-left',layouts=LAYOUTS),indent=2)+'\n')
    # Static raster PDFs are deliberately labeled as proof documents, not editable geometry.
    for stem in ['program-example']:
        run(['rsvg-convert','-f','pdf','-o',out/f'delivery-jpg-only/{stem}.pdf',out/f'delivery-jpg-only/{stem}.svg'])
    run(['ffmpeg','-v','error','-y','-i',out/'delivery-jpg-only/program-example.png','-vf','scale=960:540','-frames:v','1','-q:v','7',out/'delivery-jpg-only/client-email-960.jpg'])
    (out/'delivery-jpg-only/program-example.svg').unlink()  # genuinely raster-only handoff
    run(['ffmpeg','-v','error','-y','-loop','1','-i',speaker,'-vf',"scale=810:1080,zoompan=z='min(zoom+0.0002,1.04)':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=250:s=810x1080:fps=25",'-t','10','-an','-c:v','libx264','-pix_fmt','yuv420p','-crf','22',out/'media/video/speaker-synthetic-still-motion.mp4'])
    sequence=out/'sources/slides.ffconcat'
    save(sequence,'ffconcat version 1.0\n'+''.join(f"file '../media/slides/slide-{i:02d}.png'\nduration 5\n" for i in range(1,4))+"file '../media/slides/slide-03.png'\n")
    run(['ffmpeg','-v','error','-y','-safe','0','-f','concat','-i',sequence,'-r','25','-t','15','-an','-c:v','libx264','-pix_fmt','yuv420p','-crf','21',out/'media/video/presentation-loop.mp4'])
    run(['ffmpeg','-v','error','-y','-loop','1','-i',out/'media/backgrounds/background-only.png','-t','10','-r','25','-an','-c:v','libx264','-pix_fmt','yuv420p','-crf','22',out/'media/video/background-hold.mp4'])
    run(['ffmpeg','-v','error','-y','-loop','1','-i',out/'media/backgrounds/infodecor-empty.png','-stream_loop','-1','-i',out/'media/video/speaker-synthetic-still-motion.mp4','-i',out/'media/video/presentation-loop.mp4','-filter_complex','[1:v]scale=432:576,setsar=1[sp];[2:v]scale=1296:729,setsar=1[sl];[0:v][sp]overlay=72:260[tmp];[tmp][sl]overlay=552:260,format=yuv420p[v]','-map','[v]','-t','15','-r','25','-an','-c:v','libx264','-crf','21',out/'media/video/program-example.mp4'])
    video_checks = {}
    for name, width, height, seconds in [('speaker-synthetic-still-motion',810,1080,10),('presentation-loop',1920,1080,15),('background-hold',1920,1080,10),('program-example',1920,1080,15)]:
        result = subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(out/f'media/video/{name}.mp4')], timeout=30)
        metadata = json.loads(result); stream = metadata['streams'][0]
        if (stream['width'],stream['height']) != (width,height) or abs(float(metadata['format']['duration'])-seconds)>0.1:
            raise ValueError(f'Unexpected encoded video dimensions/duration: {name}')
        video_checks[name] = metadata
    save(out/'verification/video-metadata.json',json.dumps(video_checks,indent=2)+'\n')
    # The source text is intentionally plain text, not a second documentation system.
    save(out/'START-HERE.txt',START_HERE)
    preview=out/'delivery-jpg-only/program-example.jpg'
    save(out/'index.html',f'''<!doctype html><meta charset="utf-8"><title>Lumen Forum showcase kit</title><style>body{{margin:3em auto;max-width:1100px;background:{NAVY};color:{WHITE};font:18px/1.6 sans-serif}}a{{color:{TEAL}}}img{{width:100%;border:1px solid #315566}}small{{color:{MUTED}}}</style><h1>Lumen Forum / showcase kit</h1><p>A fictional conference. One design, two client handoffs.</p><img src="{preview.relative_to(out)}" alt="Speaker and presentation on a dark teal conference infodecor"><p><a href="START-HERE.txt">Recording guide &amp; file map</a> · <a href="delivery-jpg-only/">Raster-only delivery</a> · <a href="delivery-editable/">Editable delivery</a> · <a href="layouts/">Importable layouts</a> · <a href="media/video/program-example.mp4">Program preview video</a> · <a href="media/video/">Demo loops</a></p><small>Speaker is AI-generated. Names, event and metrics are fictional.</small>''')
    print(out)


START_HERE = '''LUMEN FORUM / CANVAS PILOT RECORDING KIT
1920 × 1080 output; all names, metrics and event content are fictional.

OPEN index.html for the visual overview. No server is needed.

01 — THE RASTER-ONLY CLIENT (WORST CASE)
Pretend the client sends only delivery-jpg-only/program-example.jpg or .png.
For a deliberately degraded delivery, use client-email-960.jpg (960 × 540).
Use Import → Reference image. Raster artwork has no editable layer geometry:
add/position rectangles manually against the image. Try lower-quality JPG vs PNG.
Reveal sources/geometry-truth.json only after the manual alignment demonstration.
The infodecor-placement-guide in media/backgrounds labels the exact pixel boxes.
background-only is the clean full background; infodecor-empty adds framed windows
and speaker labels. presentation-slide is the separate flattened presentation input.
The program PDF is a visual proof, not a supported geometry exchange format.

02 — THE EDITABLE CLIENT (BEST CASE)
Import layouts/01-speaker-and-slides/layout.drawio, .pptx, .svg, .json or .csv via
Import → Geometry file. Each contains ONLY the two named rectangles. Review and
map Speaker and Presentation. They agree to exact screen-pixel coordinates.
delivery-editable/composition.drawio, .pptx and .svg include the same two pictures
as independently movable objects plus the full-page infodecor image. Review the
background separately; do not map it as another live video layer.
The layout-only files are the least ambiguous geometry handoff.

03 — EDIT THE ACTUAL ARTWORK / PRESENTATION
presentation.pptx has three slides with editable text, shapes and chart bars.
slide-01/02/03.svg are matching editable vector slides. These are presentation
CONTENT: importing them as layout geometry will detect decorative shapes, not the
speaker/video layout. background-and-infodecor.pptx has editable title text and
shapes on two slides. composition.pptx deliberately uses a picture of slide 1;
edit the slide deck itself to change its text or chart.

04 — TWO MORE LAYOUTS
02-slide-focus: one 1728 × 972 presentation at (96,54).
03-side-by-side: two 864 × 486 windows at (72,288) and (984,288).
The ready-made project uses Centered (fit inside) for the portrait speaker.
Try intentional cropping in the 16:9 side-by-side window; do not stretch it. These are additional geometry examples,
not alternate full decorated composites.
Each layout folder includes ready-to-align.pilot.json, with no device links.
The first project embeds the composite reference with transparent layer fills.
The other projects retain the same reference hidden, ready for comparison.

05 — FAKE MEDIA FOR YOUR SWITCHER / VIDEO EDITOR
media/speaker: synthetic fictional speaker PNG (3:4).
media/slides: 3 matching 16:9 SVG/PNG/JPG slides.
media/video: 10-second silent speaker pan/zoom, 15-second silent slide loop,
a 10-second static background hold, and a 15-second combined program preview.
Speaker motion is simulated from a still;
it is not an animated talking person. Canvas Pilot itself uses still references;
play MP4 files in your switcher/media software or recording editor.

MAIN GEOMETRY (TOP-LEFT ORIGIN, PIXELS)
Speaker:      X 72,  Y 260, W 432,  H 576   (3:4)
Presentation: X 552, Y 260, W 1296, H 729   (16:9)

SUGGESTED RECORDING: show final composite → start with JPG-only handoff → manually
align two boxes → undo/reset → import native layout → compare exact coordinates →
open PPTX and change a title → show slide-focus and side-by-side → crop portrait.
Do not connect or send to a device unless separately authorized.

PROVENANCE / REBUILD
sources/provenance.json and speaker-generation-prompt.txt describe generated media.
The original layout, typography and charts are code-native. No artwork from the
reference screenshot is reused. scripts/prepare-conference-showcase.py in the repo
rebuilds the kit from the speaker PNG; it refuses to overwrite an existing folder.
Fonts: DejaVu Sans. verification/video-metadata.json records encoded video checks.
The prepared kit also includes independently rendered LibreOffice PDFs and import
checks under verification; these additional checks are not run by the generator.
'''

if __name__=='__main__':
    main()
