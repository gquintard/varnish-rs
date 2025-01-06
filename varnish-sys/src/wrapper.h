#include <sys/socket.h>
#include <sys/types.h>

#ifdef VARNISH_RS_6_0
#define FILE void *
#endif

#include "cache/cache.h"
#include "cache/cache_backend.h"
#include "cache/cache_director.h"
#include "cache/cache_filter.h"
#include "vmod_abi.h"
#include "vsb.h"
#include "vsa.h"
#include "vapi/vsm.h"
#include "vapi/vsc.h"

#ifndef VARNISH_RS_6_0
struct http_conn {
        unsigned                magic;
#define HTTP_CONN_MAGIC         0x3e19edd1
        int                     *rfd;
        stream_close_t          doclose;
        body_status_t           body_status;
        struct ws               *ws;
        char                    *rxbuf_b;
        char                    *rxbuf_e;
        char                    *pipeline_b;
        char                    *pipeline_e;
        ssize_t                 content_length;
        void                    *priv;

        /* Timeouts */
        vtim_dur                first_byte_timeout;
        vtim_dur                between_bytes_timeout;
};
#endif

struct vfp_entry *VFP_Push(struct vfp_ctx *, const struct vfp *);
