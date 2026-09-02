// Pont C entre controller-core (Rust) et l'application iOS.
//
// Toutes les fonctions écrivent dans `out` (capacité `out_cap`) et renvoient
// le nombre d'octets écrits, ou une valeur négative en cas d'erreur.
// `nexus_last_error` donne alors le détail, en UTF-8.

#ifndef NEXUS_BRIDGE_H
#define NEXUS_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NEXUS_ERR_INVALID_INPUT     (-1)
#define NEXUS_ERR_ENCODE            (-2)
#define NEXUS_ERR_DECODE            (-3)
#define NEXUS_ERR_BUFFER_TOO_SMALL  (-4)

// Requêtes sans paramètre, à passer à nexus_encode_simple_request.
typedef enum {
  NEXUS_REQ_GET_INFO      = 0,
  NEXUS_REQ_GET_CONFIG    = 1,
  NEXUS_REQ_SAVE_CONFIG   = 2,
  NEXUS_REQ_FACTORY_RESET = 3,
  NEXUS_REQ_GET_STATS     = 4,
  NEXUS_REQ_RESET_STATS   = 5,
  NEXUS_REQ_IDENTIFY      = 6,
  NEXUS_REQ_GET_BATTERY   = 7,
} NexusSimpleRequest;

ptrdiff_t nexus_last_error(uint8_t *out, size_t out_cap);
size_t    nexus_max_message_len(void);

ptrdiff_t nexus_service_uuid(uint8_t *out, size_t out_cap);
ptrdiff_t nexus_rx_char_uuid(uint8_t *out, size_t out_cap);
ptrdiff_t nexus_tx_char_uuid(uint8_t *out, size_t out_cap);

ptrdiff_t nexus_encode_simple_request(uint32_t kind, uint8_t *out, size_t out_cap);
ptrdiff_t nexus_encode_set_config(const uint8_t *json, size_t json_len,
                                  uint8_t *out, size_t out_cap);
ptrdiff_t nexus_encode_test_haptic(uint8_t effect, uint8_t *out, size_t out_cap);
ptrdiff_t nexus_encode_start_ota(const uint8_t *ssid, size_t ssid_len,
                                 const uint8_t *password, size_t password_len,
                                 const uint8_t *url, size_t url_len,
                                 uint8_t *out, size_t out_cap);

ptrdiff_t nexus_decode_response(const uint8_t *data, size_t data_len,
                                uint8_t *out, size_t out_cap);
ptrdiff_t nexus_default_config_json(uint8_t *out, size_t out_cap);
ptrdiff_t nexus_empty_stats_json(uint8_t *out, size_t out_cap);

#ifdef __cplusplus
}
#endif

#endif  // NEXUS_BRIDGE_H
