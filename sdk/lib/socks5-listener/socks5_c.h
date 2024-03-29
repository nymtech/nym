/*! \file */
/*******************************************
 *                                         *
 *  File auto-generated by `::safer_ffi`.  *
 *                                         *
 *  Do not manually edit this file.        *
 *                                         *
 *******************************************/

#ifndef __RUST_NYM_SOCKS5_LISTENER__
#define __RUST_NYM_SOCKS5_LISTENER__
#ifdef __cplusplus
extern "C" {
#endif


#include <stddef.h>
#include <stdint.h>

/** <No documentation available> */
/** \remark Has the same ABI as `uint8_t` **/
#ifdef DOXYGEN
typedef
#endif
enum ClientState {
    /** <No documentation available> */
    CLIENT_STATE_UNINITIALISED,
    /** <No documentation available> */
    CLIENT_STATE_CONNECTED,
    /** <No documentation available> */
    CLIENT_STATE_DISCONNECTED,
}
#ifndef DOXYGEN
; typedef uint8_t
#endif
ClientState_t;

/** \brief
 *  `&'lt mut (dyn 'lt + Send + FnMut(A1) -> Ret)`
 */
typedef struct RefDynFnMut1_void_char_ptr {
    /** <No documentation available> */
    void * env_ptr;

    /** <No documentation available> */
    void (*call)(void *, char *);
} RefDynFnMut1_void_char_ptr_t;

/** \brief
 *  `&'lt mut (dyn 'lt + Send + FnMut() -> Ret)`
 */
typedef struct RefDynFnMut0_void {
    /** <No documentation available> */
    void * env_ptr;

    /** <No documentation available> */
    void (*call)(void *);
} RefDynFnMut0_void_t;

/** <No documentation available> */
void
blocking_run_client (
    char const * storage_directory,
    char const * service_provider,
    RefDynFnMut1_void_char_ptr_t on_start_callback,
    RefDynFnMut0_void_t on_shutdown_callback);

/** <No documentation available> */
char *
existing_service_provider (
    char const * storage_directory);

/** <No documentation available> */
ClientState_t
get_client_state (void);

/** <No documentation available> */
void
initialise_logger (void);

/** <No documentation available> */
void
reset_client_data (
    char const * root_directory);

/** <No documentation available> */
void
rust_free_string (
    char * string);

/** <No documentation available> */
void
start_client (
    char const * storage_directory,
    char const * service_provider,
    RefDynFnMut1_void_char_ptr_t on_start_callback,
    RefDynFnMut0_void_t on_shutdown_callback);

/** <No documentation available> */
void
stop_client (void);


#ifdef __cplusplus
} /* extern \"C\" */
#endif

#endif /* __RUST_NYM_SOCKS5_LISTENER__ */
