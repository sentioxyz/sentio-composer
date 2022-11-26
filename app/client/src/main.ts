import { createApp } from 'vue';
import VueGtag from 'vue-gtag';
import App from './App.vue';
import router from './router';

createApp(App).use(router).use(VueGtag, {
  config: { id: 'G-T5DH56XEQ4' },
}).mount('#app');
