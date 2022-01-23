#include "cache/cache.h"
#include "cache/cache_filter.h"
#include "vmod_abi.h"
#include "vsb.h"

#define HTTP_CONN_MAGIC 0x3e19edd1
// for now, cherry-pick from "cache/cache_varnishd.h"
struct http_conn {
        unsigned                magic;

        int                     *rfd;
        enum sess_close         doclose;
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

struct vfp_entry *VFP_Push(struct vfp_ctx *, const struct vfp *);
