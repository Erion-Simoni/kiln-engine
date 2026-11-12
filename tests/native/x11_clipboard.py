"""Serve an ordinary X11 UTF-8 clipboard selection for the editor paste action."""
import ctypes as c, sys, time
from pathlib import Path
X=c.CDLL('libX11.so.6'); P=c.c_void_p; U=c.c_ulong; I=c.c_int
for name,args,ret in [('XOpenDisplay',[c.c_char_p],P),('XDefaultRootWindow',[P],U),('XCreateSimpleWindow',[P,U,I,I,c.c_uint,c.c_uint,c.c_uint,U,U],U),('XInternAtom',[P,c.c_char_p,I],U),('XSetSelectionOwner',[P,U,U,U],I),('XNextEvent',[P,P],I),('XChangeProperty',[P,U,U,U,I,I,P,I],I),('XSendEvent',[P,U,I,c.c_long,P],I),('XFlush',[P],I)]:
 f=getattr(X,name);f.argtypes=args;f.restype=ret
class Request(c.Structure):
 _fields_=[('type',I),('serial',U),('send_event',I),('display',P),('owner',U),('requestor',U),('selection',U),('target',U),('property',U),('time',U)]
class Notify(c.Structure):
 _fields_=[('type',I),('serial',U),('send_event',I),('display',P),('requestor',U),('selection',U),('target',U),('property',U),('time',U)]
class Event(c.Union): _fields_=[('type',I),('request',Request),('notify',Notify),('padding',c.c_long*24)]
d=X.XOpenDisplay(None)
if not d: raise RuntimeError('Cannot open X display')
w=X.XCreateSimpleWindow(d,X.XDefaultRootWindow(d),0,0,1,1,0,0,0)
atom=lambda value:X.XInternAtom(d,value.encode(),0)
clipboard,targets,utf8,string,atom_type=map(atom,['CLIPBOARD','TARGETS','UTF8_STRING','STRING','ATOM'])
payload=Path(sys.argv[1]).read_bytes();data=c.create_string_buffer(payload)
X.XSetSelectionOwner(d,clipboard,w,0);X.XFlush(d)
print('Clipboard ready',len(payload),flush=True)
while True:
 ev=Event();X.XNextEvent(d,c.byref(ev))
 if ev.type!=30: continue
 r=ev.request;prop=r.property or r.target;ok=True
 if r.target==targets:
  values=(U*3)(targets,utf8,string);X.XChangeProperty(d,r.requestor,prop,atom_type,32,0,c.cast(values,P),3)
 elif r.target in [utf8,string]: X.XChangeProperty(d,r.requestor,prop,r.target,8,0,c.cast(data,P),len(payload))
 else: ok=False
 reply=Event();reply.notify=Notify(31,0,1,d,r.requestor,r.selection,r.target,prop if ok else 0,r.time)
 X.XSendEvent(d,r.requestor,0,0,c.byref(reply));X.XFlush(d)
