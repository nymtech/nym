<script>
  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";
  import QRious from "qrious";

  const validator_urls = ["http://localhost:8080"];
  let signature;

  async function getCredential() {
    const response = await invoke("get_credential", {
      validatorUrls: validator_urls,
    });
    signature = response;
    signatureQR();
    // console.log(signature);
  }

  async function randomiseCredential() {
    const response = await invoke("randomise_credential", {
      signature: signature,
    });
    signature = response;
    signatureQR();
    // console.log(signature);
  }

  function signatureQR() {
    var qr = new QRious({
      element: document.getElementById("qr"),
      value: signature,
      foreground: "white",
      background: "black",
      size: 148
    });
  }
</script>

<svelte:head>
  <title>Coconut</title>
</svelte:head>

<button
  class={signature ? "btn btn-success" : "btn btn-danger"}
  on:click={getCredential}>Get Credential</button
>

<button
  class="btn btn-info"
  disabled={!signature}
  on:click={randomiseCredential}>Randomise Credential</button
>
<hr>

<div><canvas id="qr" /><br /></div>
