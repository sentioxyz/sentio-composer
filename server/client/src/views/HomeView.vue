<template>
  <div>
    <h1 class="head1">
      <strong>Call Aptos Function</strong>
    </h1>
    <form @submit.prevent="callFunction" class="mb-3">
      <div v-if="error" class="alert alert-dismissible alert-warning">
        <button type="button" class="close" data-dismiss="alert">×</button>
        <h4 class="alert-heading">Error!</h4>
        <p class="mb-0">{{error}}</p>
      </div>
      <div class="form-group">
        <label for="function">Function</label>
        <textarea
          v-model="message.func"
          style="width:600px;height:50px;"
          type="text"
          class="form-control"
          id="function"
          placeholder="Enter a function name, e.g. 0x1::foo::bar" required></textarea>
      </div>
      <div class="form-group">
        <label for="type_params">Type parameters</label>
        <textarea
          v-model="message.type_params"
          type="text"
          style="height:100px;"
          class="form-control"
          id="type_params"
          placeholder="Enter type parameters, seperated by ','"></textarea>
      </div>
      <div class="form-group">
        <label for="params">Parameters</label>
        <textarea
          v-model="message.params"
          type="text"
          class="form-control"
          style="height:100px;"
          id="params"
          placeholder="Enter parameters, seperated by ','"></textarea>
      </div>
      <button type="submit" class="btn btn-primary">Call Function</button>
    </form>
    <div>
        <label for="result">Result</label>
        <textarea v-show="isShow"
          readonly
          v-model="result"
          type="text"
          style="width:700px;height:200px;"
          id="result"></textarea>
    </div>
  </div>
</template>

<style>
img {
  max-width: 300px;
  height: auto;
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';

const API_URL = 'http://localhost:4000/call_function';

export default defineComponent({
  name: 'sentio-homepage',
  data: () => ({
    error: '',
    // messages: [],
    message: {
      func: '',
      type_params: '',
      params: '',
    },
    isShow: false,
    result: '',
  }),
  computed: {
    // reversedMessages() {
    //   return this.messages.slice().reverse();
    // }
  },
  // mounted() {
  //   this.resize();
  // },
  methods: {
    callFunction() {
      console.log(this.message);
      fetch(API_URL, {
        method: 'POST',
        body: JSON.stringify(this.message),
        headers: {
          'content-type': 'application/json',
        },
      })
        .then((response) => response.json())
        .then((result) => {
          if (result.error) {
            // there was an error...
            const error = 'Failed to call the function';
            this.error = error;
            this.result = error;
          } else {
            this.error = '';
            this.result = result.details;
          }
          this.isShow = true;
        });
    },
  },
});
</script>
