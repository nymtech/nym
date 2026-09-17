#!/usr/bin/env bash
#
# enable-discard.sh — make `fstrim` inside libvirt guests actually return space
# to the hypervisor, then reclaim the existing backlog.
#
# ---------------------------------------------------------------------------
# WHY THIS EXISTS
#
# qcow2 disks default to discard=ignore in QEMU. The guest is still told the
# device supports discard, so `fstrim -av` inside the VM reports gigabytes
# "trimmed" and exits 0 — while QEMU silently throws every discard away and the
# image on the host never shrinks. Operators usually discover this only after
# resorting to a nightly `virt-sparsify`, which needs the VM powered off.
#
# Setting discard='unmap' on the disk makes the guest's trims real. After that
# the guest's own fstrim.timer keeps the image compact with no downtime, and
# the sparsify cycle can stop.
#
# Confirm you are affected:
#   virsh dumpxml <vm> | grep -c "discard='unmap'"      # 0 = affected
#   qemu-img info --force-share <image>                 # qcow2 + large gap
#                                                       # between virtual and
#                                                       # disk size
# ---------------------------------------------------------------------------
#
# SAFETY
#   - backs up every domain's XML first, with a printed rollback command
#   - one VM at a time, with a configurable pause between them
#   - graceful shutdown only, with a timeout. NEVER runs `virsh destroy`
#   - restores the backup automatically if the XML edit does not apply
#   - idempotent: VMs already carrying discard='unmap' are skipped
#   - only touches a disk attribute; cannot affect networking or SSH access
#
# USAGE
#   ./enable-discard.sh --all                 # every defined VM
#   ./enable-discard.sh vm1                   # one VM (do this first)
#   ./enable-discard.sh vm1 vm2 vm3           # named VMs
#   DRY_RUN=1 ./enable-discard.sh --all       # show the plan, change nothing
#   ./enable-discard.sh --trim-only --all     # no XML change, no restart, trim only
#
# ENV
#   DRY_RUN=1             plan only
#   KEEP_GOING=1          continue past a failing VM instead of stopping
#   STAGGER=30            seconds between VMs (default 10; raise it when many
#                         guests share one array)
#   SHUTDOWN_TIMEOUT=180  seconds to wait for a clean shutdown
#   START_TIMEOUT=120     seconds to wait for the VM to run again
#   TRIM=0                skip trimming
#   TRIM_CMD='ssh root@%s fstrim -av'
#                         fallback when the guest agent is unavailable;
#                         %s is replaced with the domain name
#
# REQUIREMENTS
#   virsh, virt-xml   (apt install virtinst)
#   For automatic trimming: qemu-guest-agent inside each guest
#   (apt install qemu-guest-agent) — otherwise set TRIM_CMD, or trim by hand.

set -uo pipefail          # deliberately not -e: failures are handled and reported
export LC_ALL=C           # keep virsh output parseable regardless of locale

BACKUP_DIR="${BACKUP_DIR:-/root/libvirt-xml-backup-$(date +%Y%m%d-%H%M%S)}"
SHUTDOWN_TIMEOUT="${SHUTDOWN_TIMEOUT:-180}"
START_TIMEOUT="${START_TIMEOUT:-120}"
STAGGER="${STAGGER:-10}"
DRY_RUN="${DRY_RUN:-0}"
KEEP_GOING="${KEEP_GOING:-0}"
TRIM="${TRIM:-1}"
TRIM_CMD="${TRIM_CMD:-}"
TRIM_ONLY=0

FAILED=(); SKIPPED=(); DONE_VMS=()
TOTAL_BEFORE=0; TOTAL_AFTER=0

log()  { printf '%s\n' "$*"; }
warn() { printf '  !! %s\n' "$*" >&2; }

# ---------------------------------------------------------------- arguments --
ARGS=(); WANT_ALL=0
for a in "$@"; do
  case "$a" in
    --trim-only) TRIM_ONLY=1 ;;
    --all)       WANT_ALL=1 ;;
    -h|--help)   sed -n '2,70p' "$0"; exit 0 ;;
    -*)          echo "unknown option: $a" >&2; exit 2 ;;
    *)           ARGS+=("$a") ;;
  esac
done

command -v virsh >/dev/null || { echo "virsh not found." >&2; exit 1; }
if [[ "$TRIM_ONLY" == "0" ]]; then
  command -v virt-xml >/dev/null || {
    echo "virt-xml not found. Install it:  apt install virtinst" >&2; exit 1; }
fi

if [[ "$WANT_ALL" == "1" ]]; then
  mapfile -t VMS < <(virsh list --all --name 2>/dev/null | sed '/^$/d')
else
  VMS=("${ARGS[@]:-}")
fi
if [[ ${#VMS[@]} -eq 0 || -z "${VMS[0]:-}" ]]; then
  echo "No VMs given. Use --all or name them. See --help." >&2; exit 1
fi

# ---------------------------------------------------------------- functions --

# "target<TAB>path" per writable disk. Handles the --details layout
# (Type Device Target Source) and the plain one; skips cdrom and anything
# without a real path.
vm_disks() {
  local vm="$1" out
  out=$(virsh domblklist "$vm" --details 2>/dev/null)
  # NOTE: never pipe into `grep -q` under `set -o pipefail`. grep -q exits on the
  # first match and closes the pipe, the writer takes SIGPIPE (141), and pipefail
  # reports the whole pipeline as failed even though the match succeeded.
  # Match against a here-string instead.
  if [[ -n "$out" ]] && grep -q '[[:space:]]disk[[:space:]]' <<<"$out"; then
    printf '%s\n' "$out" | awk '$2=="disk" && $4 ~ /^\// {print $3 "\t" $4}'
    return
  fi
  virsh domblklist "$vm" 2>/dev/null | awk '$2 ~ /^\// {print $1 "\t" $2}'
}

vm_alloc_mb() {   # summed ALLOCATED MB (not apparent) across a domain's disks
  local vm="$1" total=0 path sz
  while IFS=$'\t' read -r _ path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    sz=$(du -m "$path" 2>/dev/null | awk '{print $1}')
    [[ -n "$sz" ]] && total=$((total + sz))
  done < <(vm_disks "$vm")
  printf '%s' "$total"
}

disk_format() {
  local p="$1" f
  f=$(qemu-img info --force-share "$p" 2>/dev/null | awk -F': ' '/^file format/{print $2; exit}')
  printf '%s' "${f:-unknown}"
}

has_discard() {   # 0 = every disk already has discard='unmap'
  local vm="$1" xml n_disk n_unmap
  xml=$(virsh dumpxml "$vm" 2>/dev/null)
  n_disk=$(printf '%s\n' "$xml" | grep -c "<disk type=.* device='disk'")
  n_unmap=$(printf '%s\n' "$xml" | grep -c "discard='unmap'")
  [[ "$n_disk" -gt 0 && "$n_unmap" -ge "$n_disk" ]]
}

wait_state() {   # vm, desired state, timeout, step
  local vm="$1" want="$2" limit="$3" step="${4:-5}" waited=0
  while [[ "$(virsh domstate "$vm" 2>/dev/null)" != "$want" ]]; do
    sleep "$step"; waited=$((waited + step)); printf '.'
    (( waited >= limit )) && return 1
  done
  return 0
}

guest_agent_ok() {
  virsh qemu-agent-command "$1" '{"execute":"guest-ping"}' >/dev/null 2>&1
}

trim_guest() {   # 0 if a trim actually ran
  local vm="$1" i cmd
  for i in 1 2 3 4 5 6; do
    guest_agent_ok "$vm" && break
    sleep 5
  done
  if guest_agent_ok "$vm"; then
    if virsh domfstrim "$vm" >/dev/null 2>&1; then
      log "  trimmed     : via qemu-guest-agent"; return 0
    fi
    warn "guest agent responded but domfstrim failed"
  fi
  if [[ -n "$TRIM_CMD" ]]; then
    # shellcheck disable=SC2059
    cmd=$(printf "$TRIM_CMD" "$vm")
    if eval "$cmd" >/dev/null 2>&1; then
      log "  trimmed     : via TRIM_CMD"; return 0
    fi
    warn "TRIM_CMD failed: $cmd"
  fi
  return 1
}

summary() {
  log ""
  log "=============================================================="
  log "Processed: ${#DONE_VMS[@]}   Skipped: ${#SKIPPED[@]}   Failed: ${#FAILED[@]}"
  [[ ${#SKIPPED[@]} -gt 0 ]] && log "Skipped  : ${SKIPPED[*]}"
  [[ ${#FAILED[@]}  -gt 0 ]] && log "Failed   : ${FAILED[*]}"
  if (( TOTAL_BEFORE > 0 )); then
    delta=$((TOTAL_BEFORE - TOTAL_AFTER))
    if (( delta >= 0 )); then
      log "Allocated: ${TOTAL_BEFORE} MB -> ${TOTAL_AFTER} MB (reclaimed ${delta} MB)"
    else
      # Possible on a busy node that wrote more during the run than the trim
      # returned. Not an error.
      log "Allocated: ${TOTAL_BEFORE} MB -> ${TOTAL_AFTER} MB (grew ${delta#-} MB during the run)"
    fi
  fi
  log "Backups  : $BACKUP_DIR"
  log ""
  log "Rollback for any VM:"
  log "  virsh shutdown <vm>; virsh define $BACKUP_DIR/<vm>.xml; virsh start <vm>"
  log ""
  log "Recurring trims are handled by fstrim.timer inside each guest."
  log "With discard working, nightly virt-sparsify is no longer needed."
}

fail_vm() {   # vm, message ; returns 1 when the caller should stop
  warn "$2"
  FAILED+=("$1")
  if [[ "$KEEP_GOING" == "1" ]]; then
    log "     KEEP_GOING=1 — continuing with the next VM"
    return 0
  fi
  log ""
  log "Stopping here. Re-run when resolved; completed VMs are skipped."
  log "Set KEEP_GOING=1 to process the rest regardless."
  summary
  exit 1
}

# ------------------------------------------------------------------ preflight
log "=============================================================="
log "enable-discard — ${#VMS[@]} VM(s) to consider"
[[ "$TRIM_ONLY" == "1" ]] && log "mode   : TRIM ONLY (no XML changes, no restarts)"
[[ "$DRY_RUN"   == "1" ]] && log "mode   : DRY RUN (nothing will be changed)"
log "stagger: ${STAGGER}s between VMs"
mkdir -p "$BACKUP_DIR" || { echo "cannot create $BACKUP_DIR" >&2; exit 1; }
log "backups: $BACKUP_DIR"
log "=============================================================="
log ""

# ----------------------------------------------------------------- main loop
for vm in "${VMS[@]}"; do
  [[ -z "$vm" ]] && continue
  log "=============================================================="
  log "VM: $vm"

  if ! virsh dominfo "$vm" >/dev/null 2>&1; then
    warn "no such domain — skipping"; SKIPPED+=("$vm"); continue
  fi

  mapfile -t DISKS < <(vm_disks "$vm")
  if [[ ${#DISKS[@]} -eq 0 ]]; then
    warn "no writable disks found. Raw virsh output:"
    virsh domblklist "$vm" --details 2>&1 | sed 's/^/       /'
    SKIPPED+=("$vm"); continue
  fi

  before=$(vm_alloc_mb "$vm")
  log "  disks      : ${#DISKS[@]}"
  while IFS=$'\t' read -r t p; do
    log "               $t -> $p ($(disk_format "$p"))"
  done < <(printf '%s\n' "${DISKS[@]}")
  log "  allocated  : ${before} MB"

  # ------------------------------------------------------------- trim-only --
  if [[ "$TRIM_ONLY" == "1" ]]; then
    if [[ "$DRY_RUN" == "1" ]]; then log "  DRY RUN: would trim"; continue; fi
    if [[ "$(virsh domstate "$vm" 2>/dev/null)" != "running" ]]; then
      log "  not running — cannot trim a stopped domain. Skipping."
      SKIPPED+=("$vm"); continue
    fi
    if ! has_discard "$vm"; then
      warn "discard is NOT enabled on this VM — a trim will reclaim nothing."
      log  "     Run without --trim-only first."
      SKIPPED+=("$vm"); continue
    fi
    if trim_guest "$vm"; then
      sleep 2; after=$(vm_alloc_mb "$vm")
      log "  allocated  : ${before} MB -> ${after} MB"
      TOTAL_BEFORE=$((TOTAL_BEFORE + before)); TOTAL_AFTER=$((TOTAL_AFTER + after))
      DONE_VMS+=("$vm")
    else
      log "  trim not automated — inside the guest run:  fstrim -av"
      log "  (install qemu-guest-agent in the guest, or set TRIM_CMD)"
      SKIPPED+=("$vm")
    fi
    sleep "$STAGGER"; continue
  fi

  # --------------------------------------------------------- already done --
  if has_discard "$vm"; then
    log "  discard    : already enabled — skipping"
    SKIPPED+=("$vm"); continue
  fi

  virsh dumpxml "$vm" > "$BACKUP_DIR/$vm.xml" 2>/dev/null
  if [[ ! -s "$BACKUP_DIR/$vm.xml" ]]; then
    fail_vm "$vm" "could not write XML backup — refusing to touch this VM" || continue
    continue
  fi
  log "  backed up  : $BACKUP_DIR/$vm.xml"

  if [[ "$DRY_RUN" == "1" ]]; then
    while IFS=$'\t' read -r t _; do
      log "  DRY RUN: would set discard=unmap on $t, then stop/start"
    done < <(printf '%s\n' "${DISKS[@]}")
    continue
  fi

  # ----------------------------------------------------- graceful shutdown --
  # Remember how the operator left this domain. A VM that was already shut off
  # stays shut off: --all must never boot domains somebody stopped on purpose.
  initial_state=$(virsh domstate "$vm" 2>/dev/null)
  case "$initial_state" in
    running|"shut off") ;;
    *)
      warn "state is '$initial_state' - not running or shut off. Skipping to avoid
     guessing what to do with it. Handle this VM manually."
      SKIPPED+=("$vm"); continue ;;
  esac

  if [[ "$initial_state" != "shut off" ]]; then
    printf '  shutting down'
    virsh shutdown "$vm" >/dev/null 2>&1
    if ! wait_state "$vm" "shut off" "$SHUTDOWN_TIMEOUT" 5; then
      printf '\n'
      fail_vm "$vm" "did not shut down within ${SHUTDOWN_TIMEOUT}s. NOT forcing it —
     a hard 'virsh destroy' risks filesystem and sqlite corruption. Check
     'virsh console $vm', then re-run for this VM." || continue
      continue
    fi
    printf ' done\n'
  else
    log "  state      : already shut off"
  fi

  # ------------------------------------------------------------- edit XML --
  # virt-xml preserves the other <driver> attributes (cache, io, format),
  # which a sed rewrite would not.
  edit_ok=1
  while IFS=$'\t' read -r t _; do
    virt-xml "$vm" --edit "target=$t" --disk discard=unmap >/dev/null 2>&1 || edit_ok=0
  done < <(printf '%s\n' "${DISKS[@]}")

  inactive_xml=$(virsh dumpxml --inactive "$vm" 2>/dev/null)
  if [[ "$edit_ok" == "0" ]] || ! grep -q "discard='unmap'" <<<"$inactive_xml"; then
    warn "XML edit did not apply — restoring the backup"
    virsh define "$BACKUP_DIR/$vm.xml" >/dev/null 2>&1
    virsh start "$vm" >/dev/null 2>&1
    fail_vm "$vm" "could not enable discard" || continue
    continue
  fi
  log "  XML updated: discard='unmap' on ${#DISKS[@]} disk(s)"

  # -------------------------------------------------------------- start up --
  if [[ "$initial_state" == "shut off" ]]; then
    log "  left shut off: discard is set and will take effect on next boot."
    log "                 Trim it later with:  $0 --trim-only $vm"
    DONE_VMS+=("$vm")
    log "  pausing ${STAGGER}s"
    sleep "$STAGGER"
    continue
  fi

  printf '  starting'
  virsh start "$vm" >/dev/null 2>&1
  if ! wait_state "$vm" running "$START_TIMEOUT" 3; then
    printf '\n'
    warn "did not reach running state. Restore with:"
    log  "       virsh define $BACKUP_DIR/$vm.xml && virsh start $vm"
    log  "     Console: virsh console $vm"
    fail_vm "$vm" "failed to start after the XML change" || continue
    continue
  fi
  printf ' running\n'

  # ------------------------------------------------------------------ trim --
  after="$before"
  if [[ "$TRIM" == "1" ]]; then
    if trim_guest "$vm"; then
      sleep 2; after=$(vm_alloc_mb "$vm")
      log "  allocated  : ${before} MB -> ${after} MB"
    else
      log "  trim not automated — inside the guest run:  fstrim -av"
      log "  (install qemu-guest-agent in the guest, or set TRIM_CMD)"
    fi
  fi

  TOTAL_BEFORE=$((TOTAL_BEFORE + before))
  TOTAL_AFTER=$((TOTAL_AFTER + after))
  DONE_VMS+=("$vm")

  log "  pausing ${STAGGER}s"
  sleep "$STAGGER"
done

summary
[[ ${#FAILED[@]} -eq 0 ]] || exit 1
exit 0