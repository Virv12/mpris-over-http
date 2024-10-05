class MediaWidget extends HTMLElement {
    constructor() {
        super();
    }

    connectedCallback() {
        this.child_img = document.createElement("img");
        this.appendChild(this.child_img);

        this.child_title = document.createElement("span");
        this.appendChild(this.child_title);

        this.child_progress = document.createElement("progress");
        this.appendChild(this.child_progress);

        this.eventSource = this.get_updates();

        this.addEventListener("click", () => {
            fetch(`/playpause/${this.getAttribute("media-id")}`, { method: "POST" });
        });

        this.update_timer = null;
        this.playback_rate = 1;
        this.art_url_hash = null;
    }

    disconnectedCallback() {
        this.eventSource.close();
        clearInterval(this.update_timer);
    }

    get_updates() {
        const eventSource = new EventSource(`/metadata/${this.getAttribute("media-id")}`);
        eventSource.addEventListener("update", event => {
            const data = JSON.parse(event.data);

            if (data.art_url_hash !== this.art_url_hash) {
                this.child_img.src = `/icon/${this.getAttribute("media-id")}/${data.art_url_hash}`;
                this.art_url_hash = data.art_url_hash;
            }

            this.child_title.textContent = data.title;

            this.child_progress.value = data.position;
            this.child_progress.max = data.length;

            this.playback_rate = data.playback_rate;

            if (data.running && this.update_timer === null) {
                this.update_timer = setInterval(() => {
                    this.child_progress.value += this.playback_rate;
                }, 1000);
            }
            if (!data.running && this.update_timer !== null) {
                clearInterval(this.update_timer);
                this.update_timer = null;
            }
        });
        eventSource.addEventListener("end", () => {
            this.parentNode.removeChild(this);
        });
        return eventSource;
    }
}

customElements.define("media-widget", MediaWidget);

fetch("/list")
    .then(res => res.json())
    .then(data => {
        const list = document.getElementById("list");
        for (let media of data) {
            const media_widget = document.createElement("media-widget");
            media_widget.setAttribute("media-id", media);
            list.appendChild(media_widget);
        }
    });
