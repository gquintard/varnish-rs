import optparse
import json
import io
import os
import sys

import vmodtool


#######################################################################

def conv(vt):
    s = ""
    if vt.startswith("PRIV_"):
        return "&mut " + s
    # if vt.startswith("PROBE"):
    #    return "&" + s
    elif vt == "STRING":
        return "&*" + s
    else:
        return s


def rustFuncSig(self, vcc, t):
    buf = io.StringIO()
    if t == "fini":
        buf.write("(objp: *mut *mut crate::{0})".format(self.obj[1:]))
        return buf.getvalue()
    buf.write("(vrt_ctx: * mut varnish::vcl::boilerplate::vrt_ctx")
    if t == "ini":
        buf.write(", objp: *mut *mut crate::{0}".format(self.obj[1:]))
    if t == "ini":
        buf.write(", vcl_name: *const c_char")
    if t == "meth":
        buf.write(", obj: *mut crate::{0}".format(self.obj[1:]))
    if self.argstruct:
        buf.write(", args: *const arg_{0}{1}_{2}".format(vcc.sympfx, vcc.modname, self.cname()))
    else:
        for a in self.args:
            buf.write(", {0}: {1}".format(a.nm2, a.ct))
    buf.write(")")
    if self.retval.vt != "VOID":
        buf.write(" -> {0}".format(self.retval.ct))
    return buf.getvalue()


def rustFuncArgs(self, t):
    args = []
    args.append("\t\t&mut _ctx")
    if t == "ini":
        args.append("\t\t&vcl_name.into_rust()")
    for a in self.args:
        if self.argstruct:
            if a.opt:
                args.append(
                    "\t\tif (*args).valid_{nm} == 0 {{ None }} else {{ Some({conv}arg_{nm}) }}".format(conv=conv(a.vt),
                                                                                                       nm=a.nm2))
            else:
                args.append("\t\t{conv}arg_{nm}".format(conv=conv(a.vt), nm=a.nm2))
        else:
            args.append("\t\t{conv}arg_{nm}".format(conv=conv(a.vt), nm=a.nm2))
    print(",\n".join(args))


def rustfuncBody(self, vcc, t):
    if self.argstruct:
        print("#[repr(C)]\nstruct arg_{0}{1}_{2} {{".format(vcc.sympfx, vcc.modname, self.cname()))
        for a in self.args:
            if a.opt:
                assert a.nm is not None
                print("\tvalid_{0}: c_char,".format(a.nm))
        for a in self.args:
            print("\t{0}: {1},".format(a.nm2, a.ct))
        print("}\n")

    print("unsafe extern \"C\" fn vmod_c_{0}{1} {{".format(self.cname(), rustFuncSig(self, vcc, t)))
    if (t != "fini"):
        print("\tlet mut _ctx = Ctx::from_ptr(vrt_ctx);");
    for a in self.args:
        if self.argstruct:
            print("\tlet mut arg_{nm} = (*args).{nm}.into_rust();".format(nm=a.nm2))
        else:
            print("\tlet mut arg_{nm} = {nm}.into_rust();".format(nm=a.nm2))
    if t == "ini":
        print("\tmatch crate::{0}::new(".format(self.obj[1:]))
        rustFuncArgs(self, t)
        print('''\t) {
\t\tOk(o) => { *objp = Box::into_raw(Box::new(o)); },
\t\tErr(e) => { _ctx.fail(&e.to_string()); },
\t}''')
    elif t == "fini":
        print("\tdrop(Box::from_raw(*objp));")
    else:
        if t == "meth":
            print("\t(*obj){name}(".format(name=self.bname))
        else:
            print("\tcrate::{name}(".format(name=self.cname()))
        rustFuncArgs(self, t)
        print("\t).into_result().and_then(|v| v.into_vcl(&mut _ctx.ws)).unwrap_or_else(|e| {{ _ctx.fail(&e); <{0}>::vcl_default() }})".format(self.retval.ct if self.retval.vt != "VOID" else "()"))
    print("}")


def rustEventFunc():
    print('''
unsafe extern "C" fn vmod_c__event(vrt_ctx: * mut varnish::vcl::boilerplate::vrt_ctx, vp: *mut varnish::vcl::boilerplate::vmod_priv, ev: varnish::vcl::boilerplate::vcl_event_e) -> varnish::vcl::boilerplate::VCL_INT {
    let mut ctx = Ctx::from_ptr(vrt_ctx);
    let event = Event::new(ev);
    match crate::event(
        &mut ctx,
        &mut vp.into_rust(),
        event).into_result() {
            Ok(()) => 0,
            Err(ref e) => {{ ctx.fail(e); 1 }},
    }
}
''')


def runmain(inputvcc, rstdir, abi):
    v = vmodtool.vcc(inputvcc, rstdir, None)
    v.parse()

    v.commit()

    buf = io.StringIO()

    v.mkdefs(buf);
    for i in v.contents:
        if isinstance(i, vmodtool.ObjectStanza):
            i.cstuff(buf, 'o')

    buf.write("/* Functions */\n")
    for i in v.contents:
        if isinstance(i, vmodtool.FunctionStanza):
            i.cstuff(buf, 'o')

    v.cstruct(buf)

    buf.write('#undef VPFX\n')
    buf.write('#undef VARGS\n')
    buf.write('#undef VENUM\n')

    proto = buf.getvalue() + "static struct {csn} {csn};".format(csn=v.csn)
    buf.close()

    print("""
use std::ptr;
use std::ffi::*;
use std::boxed::Box;
use varnish::vcl::ctx::{{ Ctx, Event }};
use varnish::vcl::convert::{{IntoRust, IntoVCL, IntoResult, VCLDefault}};
""".format(modname=v.modname))

    # C stuff is done, get comfortable with our own types
    for i in vmodtool.CTYPES:
        if i.startswith("PRIV_"):
            vmodtool.CTYPES[i] = "*mut varnish::vcl::boilerplate::vmod_priv"
        else:
            vmodtool.CTYPES[i] = "varnish::vcl::boilerplate::" + vmodtool.CTYPES[i]
    v = vmodtool.vcc(inputvcc, None, None)
    v.parse()

    for i in v.contents:
        if isinstance(i, vmodtool.FunctionStanza):
            rustfuncBody(i.proto, v, "func")
        elif isinstance(i, vmodtool.ObjectStanza):
            rustfuncBody(i.init, v, "ini")
            rustfuncBody(i.fini, v, "fini")
            for m in i.methods:
                rustfuncBody(m.proto, v, "meth")
        elif isinstance(i, vmodtool.EventStanza):
            rustEventFunc()

    print("""
#[repr(C)]
pub struct {csn} {{""".format(csn=v.csn))
    for i in v.contents:
        def rustMemberDeclare(name, func, t):
            print("\t{0}:\tOption<unsafe extern \"C\" fn{1}>,".format(name, rustFuncSig(func, v, t)))

        if isinstance(i, vmodtool.EventStanza):
            print(
                '''\t_event: Option<unsafe extern "C" fn(vrt_ctx: * mut varnish::vcl::boilerplate::vrt_ctx, vp: *mut varnish::vcl::boilerplate::vmod_priv, ev: varnish::vcl::boilerplate::vcl_event_e) -> varnish::vcl::boilerplate::VCL_INT>,''')
        elif isinstance(i, vmodtool.FunctionStanza):
            rustMemberDeclare(i.proto.cname(), i.proto, "func")
        elif isinstance(i, vmodtool.ObjectStanza):
            rustMemberDeclare(i.init.cname(), i.init, "ini")
            rustMemberDeclare(i.fini.cname(), i.fini, "fini")
            for m in i.methods:
                rustMemberDeclare(m.proto.cname(), m.proto, "meth")
    print("}")

    print("""
#[no_mangle]
pub static {csn}: {csn} = {csn} {{""".format(csn=v.csn))
    for i in v.contents:
        def rustMemberAssign(name):
            print("\t{0}: Some(vmod_c_{0}),".format(name))

        if isinstance(i, vmodtool.EventStanza):
            print("\t_event: Some(vmod_c__event),")
        elif isinstance(i, vmodtool.FunctionStanza):
            rustMemberAssign(i.proto.cname())
        elif isinstance(i, vmodtool.ObjectStanza):
            rustMemberAssign(i.init.cname())
            rustMemberAssign(i.fini.cname())
            for m in i.methods:
                rustMemberAssign(m.proto.cname())
    print("};")

    if v.strict_abi:
        major = "0"
        minor = "0"
    else:
        major = "varnish::vcl::boilerplate::VRT_MAJOR_VERSION"
        minor = "varnish::vcl::boilerplate::VRT_MINOR_VERSION"

    jl = [["$VMOD", "1.0", v.modname, v.csn, v.file_id, abi, major, minor]]
    jl.append(["$CPROTO", proto])
    for j in v.contents:
        j.json(jl)

    print("""
#[repr(C)]
pub struct vmod_data {{
	vrt_major: c_uint,
	vrt_minor: c_uint,
	file_id: *const c_char,
	name: *const c_char,
	func_name: *const c_char,
	func: *const c_void,
	func_len: c_int,
	proto: *const c_char,
	json: *const c_char,
	abi: *const c_char,
}}
unsafe impl Sync for vmod_data {{}}

#[no_mangle]
pub static Vmod_{name}_Data: vmod_data = vmod_data {{
	vrt_major: {major},
	vrt_minor: {minor},
	file_id: c"{file_id}".as_ptr(),
	name: c"{name}".as_ptr(),
	func_name: c"{csn}".as_ptr(),
	func_len: ::std::mem::size_of::<{csn}>() as c_int,
	func: &{csn} as *const _ as *const c_void,
	abi: varnish::vcl::boilerplate::VMOD_ABI_Version.as_ptr(),
	json: JSON,
    proto: std::ptr::null(),
}};

const JSON: *const c_char =
    c"VMOD_JSON_SPEC\x02\\n{json}\\n\x03".as_ptr();
""".format(
        file_id=v.file_id,
        name=v.modname,
        csn=v.csn,
        major=major,
        minor=minor,
        json=escape_json(json.dumps(jl, indent=4))
    ))


def escape_json(s):
    n = ""
    for i in s:
        if i in '"\\':
            n += '\\'
        n += i
    return n


if __name__ == "__main__":
    usagetext = "Usage: %prog [options] <vmod.vcc>"
    oparser = optparse.OptionParser(usage=usagetext)

    oparser.add_option('-N', '--strict', action='store_true', default=False,
                       help="Be strict when parsing the input file")
    oparser.add_option('-w', '--rstdir', metavar="directory", default='.',
                       help='Where to save the generated RST files ' +
                            '(default: ".")')
    oparser.add_option('-a', '--abi',
                       help='abi string (VMOD_ABI_Version in C)')
    (opts, args) = oparser.parse_args()

    i_vcc = None
    for f in args:
        if os.path.exists(f):
            i_vcc = f
            break
    if i_vcc is None and os.path.exists("vmod.vcc"):
        i_vcc = "vmod.vcc"
    if i_vcc is None:
        print("ERROR: No vmod.vcc file supplied or found.", file=sys.stderr)
        oparser.print_help()
        exit(-1)

    runmain(i_vcc, opts.rstdir, opts.abi)
