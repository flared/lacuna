// Layers — must be first to establish cascade order
import "@/styles/layers.css";

// Composables
import { createApp } from "vue";

// Plugins
import { registerPlugins } from "@/plugins";

// Components
import App from "@/App.vue";

// Styles
import "unfonts.css";
import "@/styles/tailwind.css";
import "@/styles/main.scss";

const app = createApp(App);

registerPlugins(app);

app.mount("#app");
