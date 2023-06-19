#define BACDL_ALL 1
#define BACDL_BIP 1
#define BACAPP_PRINT_ENABLED 1
#include "bacnet-stack/src/bacnet/bactext.h"
#include "bacnet-stack/src/bacnet/iam.h"
#include "bacnet-stack/src/bacnet/ihave.h"
#include "bacnet-stack/src/bacnet/basic/binding/address.h"
#include "bacnet-stack/src/bacnet/config.h"
#include "bacnet-stack/src/bacnet/bacdef.h"
#include "bacnet-stack/src/bacnet/npdu.h"
#include "bacnet-stack/src/bacnet/apdu.h"
#include "bacnet-stack/src/bacnet/proplist.h"
#include "bacnet-stack/src/bacnet/property.h"
#include "bacnet-stack/src/bacnet/basic/services.h"
#include "bacnet-stack/src/bacnet/basic/object/device.h"
#include "bacnet-stack/src/bacnet/basic/tsm/tsm.h"
#include "bacnet-stack/src/bacnet/datalink/datalink.h"
#include "bacnet-stack/src/bacnet/version.h"
#include "bacnet-stack/src/bacnet/datalink/dlenv.h"
#include "bacnet-stack/src/bacnet/bacenum.h"
// #include "bacnet-stack/src/bacnet/bacport.h"

// #include "bacnet-stack/src/bacnet/abort.h"
// #include "bacnet-stack/src/bacnet/access_rule.h"
// #include "bacnet-stack/src/bacnet/alarm_ack.h"
// #include "bacnet-stack/src/bacnet/apdu.h"
// #include "bacnet-stack/src/bacnet/arf.h"
// #include "bacnet-stack/src/bacnet/assigned_access_rights.h"
// #include "bacnet-stack/src/bacnet/authentication_factor.h"
// #include "bacnet-stack/src/bacnet/authentication_factor_format.h"
// #include "bacnet-stack/src/bacnet/awf.h"
// #include "bacnet-stack/src/bacnet/bacaddr.h"
// #include "bacnet-stack/src/bacnet/bacapp.h"
// #include "bacnet-stack/src/bacnet/bacdcode.h"
// #include "bacnet-stack/src/bacnet/bacdef.h"
// #include "bacnet-stack/src/bacnet/bacdest.h"
// #include "bacnet-stack/src/bacnet/bacdevobjpropref.h"
// #include "bacnet-stack/src/bacnet/bacenum.h"
// #include "bacnet-stack/src/bacnet/bacerror.h"
// #include "bacnet-stack/src/bacnet/bacint.h"
// #include "bacnet-stack/src/bacnet/bacnet_stack_exports.h"
// #include "bacnet-stack/src/bacnet/bacprop.h"
// #include "bacnet-stack/src/bacnet/bacpropstates.h"
// #include "bacnet-stack/src/bacnet/bacreal.h"
// #include "bacnet-stack/src/bacnet/bacstr.h"
// #include "bacnet-stack/src/bacnet/bactext.h"
// #include "bacnet-stack/src/bacnet/bactimevalue.h"
// #include "bacnet-stack/src/bacnet/bits.h"
// #include "bacnet-stack/src/bacnet/bytes.h"
// #include "bacnet-stack/src/bacnet/config.h"
// #include "bacnet-stack/src/bacnet/cov.h"
// #include "bacnet-stack/src/bacnet/credential_authentication_factor.h"
// #include "bacnet-stack/src/bacnet/dailyschedule.h"
// #include "bacnet-stack/src/bacnet/datetime.h"
// #include "bacnet-stack/src/bacnet/dcc.h"
// #include "bacnet-stack/src/bacnet/event.h"
// #include "bacnet-stack/src/bacnet/get_alarm_sum.h"
// #include "bacnet-stack/src/bacnet/getevent.h"
// #include "bacnet-stack/src/bacnet/hostnport.h"
// #include "bacnet-stack/src/bacnet/iam.h"
// #include "bacnet-stack/src/bacnet/ihave.h"
// #include "bacnet-stack/src/bacnet/indtext.h"
// #include "bacnet-stack/src/bacnet/lighting.h"
// #include "bacnet-stack/src/bacnet/list_element.h"
// #include "bacnet-stack/src/bacnet/lso.h"
// #include "bacnet-stack/src/bacnet/memcopy.h"
// #include "bacnet-stack/src/bacnet/npdu.h"
// #include "bacnet-stack/src/bacnet/property.h"
// #include "bacnet-stack/src/bacnet/proplist.h"
// #include "bacnet-stack/src/bacnet/ptransfer.h"
// #include "bacnet-stack/src/bacnet/rd.h"
// #include "bacnet-stack/src/bacnet/readrange.h"
// #include "bacnet-stack/src/bacnet/reject.h"
// #include "bacnet-stack/src/bacnet/rp.h"
// #include "bacnet-stack/src/bacnet/rpm.h"
// #include "bacnet-stack/src/bacnet/timestamp.h"
// #include "bacnet-stack/src/bacnet/timesync.h"
// #include "bacnet-stack/src/bacnet/version.h"
// #include "bacnet-stack/src/bacnet/weeklyschedule.h"
// #include "bacnet-stack/src/bacnet/whohas.h"
// #include "bacnet-stack/src/bacnet/whois.h"
// #include "bacnet-stack/src/bacnet/wp.h"
// #include "bacnet-stack/src/bacnet/wpm.h"

// #include "bacnet-stack/src/bacnet/datalink/arcnet.h"
// #include "bacnet-stack/src/bacnet/datalink/bacsec.h"
// #include "bacnet-stack/src/bacnet/datalink/bip.h"
// #include "bacnet-stack/src/bacnet/datalink/bip6.h"
// #include "bacnet-stack/src/bacnet/datalink/bvlc.h"
// #include "bacnet-stack/src/bacnet/datalink/bvlc6.h"
// #include "bacnet-stack/src/bacnet/datalink/cobs.h"
// #include "bacnet-stack/src/bacnet/datalink/crc.h"
// #include "bacnet-stack/src/bacnet/datalink/datalink.h"
// #include "bacnet-stack/src/bacnet/datalink/dlenv.h"
// #include "bacnet-stack/src/bacnet/datalink/dlmstp.h"
// #include "bacnet-stack/src/bacnet/datalink/ethernet.h"
// #include "bacnet-stack/src/bacnet/datalink/mstp.h"
// #include "bacnet-stack/src/bacnet/datalink/mstpdef.h"
// #include "bacnet-stack/src/bacnet/datalink/mstptext.h"

// #include "bacnet-stack/src/bacnet/basic/services.h"

// #include "bacnet-stack/src/bacnet/basic/bbmd/h_bbmd.h"

// #include "bacnet-stack/src/bacnet/basic/bbmd6/h_bbmd6.h"
// #include "bacnet-stack/src/bacnet/basic/bbmd6/vmac.h"

// #include "bacnet-stack/src/bacnet/basic/binding/address.h"

// #include "bacnet-stack/src/bacnet/basic/client/bac-data.h"
// #include "bacnet-stack/src/bacnet/basic/client/bac-rw.h"
// #include "bacnet-stack/src/bacnet/basic/client/bac-task.h"

// #include "bacnet-stack/src/bacnet/basic/npdu/h_npdu.h"
// #include "bacnet-stack/src/bacnet/basic/npdu/h_routed_npdu.h"
// #include "bacnet-stack/src/bacnet/basic/npdu/s_router.h"

// #include "bacnet-stack/src/bacnet/basic/sys/bigend.h"
// #include "bacnet-stack/src/bacnet/basic/sys/color_rgb.h"
// #include "bacnet-stack/src/bacnet/basic/sys/days.h"
// #include "bacnet-stack/src/bacnet/basic/sys/debug.h"
// #include "bacnet-stack/src/bacnet/basic/sys/fifo.h"
// #include "bacnet-stack/src/bacnet/basic/sys/filename.h"
// #include "bacnet-stack/src/bacnet/basic/sys/key.h"
#include "bacnet-stack/src/bacnet/basic/sys/keylist.h"
// #include "bacnet-stack/src/bacnet/basic/sys/mstimer.h"
// #include "bacnet-stack/src/bacnet/basic/sys/platform.h"
// #include "bacnet-stack/src/bacnet/basic/sys/ringbuf.h"
// #include "bacnet-stack/src/bacnet/basic/sys/sbuf.h"

#include "bacnet-stack/src/bacnet/basic/object/acc.h"
#include "bacnet-stack/src/bacnet/basic/object/access_credential.h"
#include "bacnet-stack/src/bacnet/basic/object/access_door.h"
#include "bacnet-stack/src/bacnet/basic/object/access_point.h"
#include "bacnet-stack/src/bacnet/basic/object/access_rights.h"
#include "bacnet-stack/src/bacnet/basic/object/access_user.h"
#include "bacnet-stack/src/bacnet/basic/object/access_zone.h"
#include "bacnet-stack/src/bacnet/basic/object/ai.h"
#include "bacnet-stack/src/bacnet/basic/object/ao.h"
#include "bacnet-stack/src/bacnet/basic/object/av.h"
#include "bacnet-stack/src/bacnet/basic/object/bacfile.h"
#include "bacnet-stack/src/bacnet/basic/object/bi.h"
#include "bacnet-stack/src/bacnet/basic/object/bo.h"
#include "bacnet-stack/src/bacnet/basic/object/bv.h"
#include "bacnet-stack/src/bacnet/basic/object/channel.h"
#include "bacnet-stack/src/bacnet/basic/object/color_object.h"
#include "bacnet-stack/src/bacnet/basic/object/color_temperature.h"
#include "bacnet-stack/src/bacnet/basic/object/command.h"
#include "bacnet-stack/src/bacnet/basic/object/credential_data_input.h"
#include "bacnet-stack/src/bacnet/basic/object/csv.h"
#include "bacnet-stack/src/bacnet/basic/object/device.h"
#include "bacnet-stack/src/bacnet/basic/object/iv.h"
#include "bacnet-stack/src/bacnet/basic/object/lc.h"
#include "bacnet-stack/src/bacnet/basic/object/lo.h"
#include "bacnet-stack/src/bacnet/basic/object/lsp.h"
#include "bacnet-stack/src/bacnet/basic/object/ms-input.h"
#include "bacnet-stack/src/bacnet/basic/object/mso.h"
#include "bacnet-stack/src/bacnet/basic/object/msv.h"
#include "bacnet-stack/src/bacnet/basic/object/nc.h"
#include "bacnet-stack/src/bacnet/basic/object/netport.h"
#include "bacnet-stack/src/bacnet/basic/object/objects.h"
#include "bacnet-stack/src/bacnet/basic/object/osv.h"
#include "bacnet-stack/src/bacnet/basic/object/piv.h"
#include "bacnet-stack/src/bacnet/basic/object/schedule.h"
#include "bacnet-stack/src/bacnet/basic/object/trendlog.h"

// #include "bacnet-stack/src/bacnet/basic/service/h_alarm_ack.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_apdu.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_arf.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_arf_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_awf.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_ccov.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_cov.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_dcc.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_gas_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_get_alarm_sum.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_getevent.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_getevent_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_iam.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_ihave.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_list_element.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_lso.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_noserv.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rd.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rp.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rp_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rpm.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rpm_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rr.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_rr_a.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_ts.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_ucov.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_upt.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_whohas.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_whois.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_wp.h"
// #include "bacnet-stack/src/bacnet/basic/service/h_wpm.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_abort.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_ack_alarm.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_arfs.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_awfs.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_cevent.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_cov.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_dcc.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_error.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_get_alarm_sum.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_get_event.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_getevent.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_iam.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_ihave.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_list_element.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_lso.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_rd.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_readrange.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_rp.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_rpm.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_ts.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_uevent.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_upt.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_whohas.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_whois.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_wp.h"
// #include "bacnet-stack/src/bacnet/basic/service/s_wpm.h"

// #include "bacnet-stack/src/bacnet/basic/tsm/tsm.h"

// #include "bacnet-stack/src/bacnet/basic/ucix/ucix.h"
