/**************************************************************************
*
* Copyright (C) 2005-2006 Steve Karg <skarg@users.sourceforge.net>
*
* Permission is hereby granted, free of charge, to any person obtaining
* a copy of this software and associated documentation files (the
* "Software"), to deal in the Software without restriction, including
* without limitation the rights to use, copy, modify, merge, publish,
* distribute, sublicense, and/or sell copies of the Software, and to
* permit persons to whom the Software is furnished to do so, subject to
* the following conditions:
*
* The above copyright notice and this permission notice shall be included
* in all copies or substantial portions of the Software.
*
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
* EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
* MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
* IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
* CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
* TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
* SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*
*********************************************************************/
#ifndef BASIC_SERVICES_H
#define BASIC_SERVICES_H

/* NPDU layer handlers */
#include "npdu/h_npdu.h"
#include "npdu/h_routed_npdu.h"
#include "npdu/s_router.h"

/* application layer binding handler */
#include "binding/address.h"

/* application layer service handler */
#include "service/h_alarm_ack.h"
#include "service/h_apdu.h"
#include "service/h_arf.h"
#include "service/h_arf_a.h"
#include "service/h_awf.h"
#include "service/h_ccov.h"
#include "service/h_cov.h"
#include "service/h_dcc.h"
#include "service/h_gas_a.h"
#include "service/h_get_alarm_sum.h"
#include "service/h_getevent.h"
#include "service/h_getevent_a.h"
#include "service/h_iam.h"
#include "service/h_ihave.h"
#include "service/h_list_element.h"
#include "service/h_lso.h"
#include "service/h_noserv.h"
#include "service/h_rd.h"
#include "service/h_rp.h"
#include "service/h_rp_a.h"
#include "service/h_rpm.h"
#include "service/h_rpm_a.h"
#include "service/h_rr.h"
#include "service/h_rr_a.h"
#include "service/h_ts.h"
#include "service/h_ucov.h"
#include "service/h_upt.h"
#include "service/h_whohas.h"
#include "service/h_whois.h"
#include "service/h_wp.h"
#include "service/h_wpm.h"

/* application layer service send helpers */
#include "service/s_abort.h"
#include "service/s_ack_alarm.h"
#include "service/s_arfs.h"
#include "service/s_awfs.h"
#include "service/s_cevent.h"
#include "service/s_cov.h"
#include "service/s_dcc.h"
#include "service/s_error.h"
#include "service/s_get_alarm_sum.h"
#include "service/s_get_event.h"
#include "service/s_getevent.h"
#include "service/s_iam.h"
#include "service/s_ihave.h"
#include "service/s_list_element.h"
#include "service/s_lso.h"
#include "service/s_rd.h"
#include "service/s_readrange.h"
#include "service/s_rp.h"
#include "service/s_rpm.h"
#include "service/s_ts.h"
#include "service/s_uevent.h"
#include "service/s_upt.h"
#include "service/s_whohas.h"
#include "service/s_whois.h"
#include "service/s_wp.h"
#include "service/s_wpm.h"

/** @defgroup MISCHNDLR Miscellaneous Service Handlers
 * Various utilities and functions to support the service handlers
 */
#endif
