import Vue from "vue";
import Vuex from "vuex";
import cosmos from "@tendermint/vue/src/store/cosmos.js";

Vue.use(Vuex);

export default new Vuex.Store({
  modules: { cosmos },
});
