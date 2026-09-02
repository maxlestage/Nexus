// Frontière C du cœur de l'application (Rust).
//
// Swift ne détient que deux choses : la liaison CoreBluetooth et le rendu.
// Tout le reste — état, actions, libellés, description de l'interface,
// protocole — vit derrière cette interface.
//
// Convention : les fonctions qui produisent des octets écrivent dans `out`
// (capacité `out_cap`) et renvoient le nombre d'octets écrits, ou une valeur
// négative en cas d'erreur ; `nexus_last_error` donne alors le détail.

#ifndef NEXUS_BRIDGE_H
#define NEXUS_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NEXUS_ERR_INVALID_INPUT     (-1)
#define NEXUS_ERR_ENCODE            (-2)
#define NEXUS_ERR_BUFFER_TOO_SMALL  (-4)
#define NEXUS_NOTHING_TO_SEND       (0)

// Événements de liaison remontés depuis CoreBluetooth.
typedef enum {
  NEXUS_BLE_SCANNING     = 0,
  NEXUS_BLE_CONNECTING   = 1,
  NEXUS_BLE_READY        = 2,
  NEXUS_BLE_DISCONNECTED = 3,
  NEXUS_BLE_OFF          = 4,
  NEXUS_BLE_UNAUTHORIZED = 5,
  NEXUS_BLE_UNSUPPORTED  = 6,
} NexusBleEvent;

typedef struct NexusApp NexusApp;

NexusApp *nexus_app_new(void);
void      nexus_app_free(NexusApp *state);

ptrdiff_t nexus_last_error(uint8_t *out, size_t out_cap);
size_t    nexus_max_message_len(void);

// Interface : modèle de vue en JSON, et actions de l'utilisateur.
ptrdiff_t nexus_app_view(NexusApp *state, uint8_t *out, size_t out_cap);
ptrdiff_t nexus_app_dispatch(NexusApp *state,
                             const uint8_t *id, size_t id_len,
                             const uint8_t *value_json, size_t value_len);

// Bluetooth : événements, données reçues, trames à émettre.
ptrdiff_t nexus_app_ble_event(NexusApp *state, uint32_t code);
ptrdiff_t nexus_app_ble_data(NexusApp *state, const uint8_t *data, size_t data_len);
ptrdiff_t nexus_app_ble_error(NexusApp *state, const uint8_t *message, size_t message_len);
ptrdiff_t nexus_app_take_outgoing(NexusApp *state, uint8_t *out, size_t out_cap);

// Identifiants du service BLE, définis par le protocole partagé.
ptrdiff_t nexus_service_uuid(uint8_t *out, size_t out_cap);
ptrdiff_t nexus_rx_char_uuid(uint8_t *out, size_t out_cap);
ptrdiff_t nexus_tx_char_uuid(uint8_t *out, size_t out_cap);

#ifdef __cplusplus
}
#endif

#endif  // NEXUS_BRIDGE_H
