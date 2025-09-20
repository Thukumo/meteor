// ===================== 定数・要素取得 =====================
const roomNameInput = document.getElementById('roomNameInput');
const connectButton = document.getElementById('connectButton');
const connectionStatus = document.getElementById('connectionStatus');
const allowFlowCheckbox = document.getElementById('allowFlowCheckbox');
const startScreenShareButton = document.getElementById('startScreenShareButton');
const screenShareVideo = document.getElementById('screenShareVideo');
const commentCanvas = document.getElementById('commentCanvas');
const ctx = commentCanvas.getContext('2d');
const uiContainer = document.querySelector('.container');
const fontSizeSlider = document.getElementById('fontSizeSlider');
const fontSizeDisplay = document.getElementById('fontSizeDisplay');

let ws = null;
let reconnectTimeout = null;
let reconnectAttempts = 0;
const RECONNECT_MAX_ATTEMPTS = 10;
const RECONNECT_BASE_MS = 500;
const activeComments = [];
const availableColors = ['red', 'orange', 'skyblue', 'springgreen', 'mediumpurple', 'mediumblue'];
const COMMENT_DURATION_MS = 6000;

// ===================== ユーティリティクラス =====================
class CustomRandom {
    constructor(seed, start, end) {
        this.start = start;
        this.end = end;
        this.lastValue = 0;
    }
    getRandomValue() {
        let value;
        do {
            value = Math.floor(Math.random() * (this.end - this.start)) + this.start;
        } while (value === this.lastValue);
        this.lastValue = value;
        return value;
    }
    getNotNearlyRandomValue() {
        let value;
        do {
            value = Math.floor(Math.random() * (this.end - this.start)) + this.start;
        } while (Math.abs(value - this.lastValue) == 0);
        this.lastValue = value;
        return value;
    }
}
const colorRandom = new CustomRandom(1000, 0, availableColors.length);
const yPosRandom = new CustomRandom(1000, 0, 10);

// ===================== 接続状態UI管理 =====================
function setConnectionUI(state) {
    // state: 'idle' | 'connecting' | 'connected' | 'disconnected' | 'error'
    switch (state) {
        case 'connecting':
            connectionStatus.textContent = '接続中...';
            connectionStatus.className = 'status-message';
            connectButton.textContent = '接続中...';
            connectButton.style.backgroundColor = '#007bff';
            roomNameInput.disabled = false; // 接続試行中も編集可
            break;
        case 'connected':
            connectionStatus.textContent = '接続済';
            connectionStatus.className = 'status-message connected';
            connectButton.textContent = '接続済';
            connectButton.style.backgroundColor = 'green';
            roomNameInput.disabled = false; // 接続中でも編集可
            break;
        case 'disconnected':
            connectionStatus.textContent = '未接続';
            connectionStatus.className = 'status-message disconnected';
            connectButton.textContent = '接続開始';
            connectButton.style.backgroundColor = '#007bff';
            roomNameInput.disabled = false;
            break;
        case 'error':
            connectionStatus.textContent = 'WebSocket接続エラー';
            connectionStatus.className = 'status-message error';
            connectButton.textContent = '再試行';
            connectButton.style.backgroundColor = '#007bff';
            roomNameInput.disabled = false;
            break;
        default:
            connectionStatus.textContent = '未接続';
            connectionStatus.className = 'status-message';
            connectButton.textContent = '接続開始';
            connectButton.style.backgroundColor = '#007bff';
            roomNameInput.disabled = false;
    }
}

// ===================== WebSocket接続 =====================
function connectWebSocket(roomName) {
    // 既存の再接続タイマーをクリア
    if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
        reconnectTimeout = null;
    }
    // 既存の接続が生きている/接続中の場合は何もしない
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
        return;
    }
    // 新規接続開始
    setConnectionUI('connecting');
    ws = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}/api/v1/room/${encodeURIComponent(roomName)}/ws`);
    reconnectAttempts = 0;
    ws.onopen = () => {
        setConnectionUI('connected');
        if (reconnectTimeout) { clearTimeout(reconnectTimeout); reconnectTimeout = null; }
    };
    ws.onmessage = (event) => {
        const commentText = event.data;
        if (commentText && commentText.trim() !== "") addCommentToCanvas(commentText);
    };
    ws.onclose = () => {
        setConnectionUI('disconnected');
        // Exponential backoff with jitter, up to 30s
        reconnectAttempts += 1;
        if (reconnectAttempts > RECONNECT_MAX_ATTEMPTS) return;
        const expo = Math.min(RECONNECT_BASE_MS * Math.pow(2, reconnectAttempts - 1), 30000);
        const jitter = Math.random() * 300;
        const delay = Math.floor(expo + jitter);
        reconnectTimeout = setTimeout(() => {
            // 入力欄が変更されていたら再接続せず停止
            const current = roomNameInput.value.trim();
            if (current === roomName) {
                connectWebSocket(roomName);
            } else {
                setConnectionUI('disconnected');
            }
        }, delay);
    };
    ws.onerror = (error) => {
        setConnectionUI('error');
        // 連続で多発しないようにアラートは一度だけ
        if (reconnectAttempts === 0) {
            alert("WebSocket接続エラーが発生しました。");
        }
        try { ws.close(); } catch (_) {}
    };
}

// ===================== コメント描画 =====================
function addCommentToCanvas(commentText) {
    if (!allowFlowCheckbox.checked) return;
    let desiredFontSize = parseInt(fontSizeSlider.value, 10);
    if (isNaN(desiredFontSize) || desiredFontSize < parseInt(fontSizeSlider.min, 10)) {
        desiredFontSize = parseInt(fontSizeSlider.min, 10);
    }
    const font = `bold ${desiredFontSize}px sans-serif`;
    const canvasHeight = commentCanvas.height;
    const effectiveCommentHeight = desiredFontSize + 10;
    const numRows = Math.floor(canvasHeight / effectiveCommentHeight);
    const actualNumRows = Math.max(1, numRows);
    const randomYPosRow = yPosRandom.getNotNearlyRandomValue() % actualNumRows;
    const startY = randomYPosRow * effectiveCommentHeight + (desiredFontSize * 0.7);
    const color = availableColors[colorRandom.getRandomValue()];
    ctx.font = font;
    const comment = {
        text: commentText,
        x: commentCanvas.width,
        y: startY,
        color: color,
        font: font,
        startTime: performance.now(),
        duration: COMMENT_DURATION_MS,
        width: ctx.measureText(commentText).width
    };
    activeComments.push(comment);
}

function animateCanvasComments() {
    if (screenShareVideo.readyState >= 2) {
        commentCanvas.width = screenShareVideo.videoWidth;
        commentCanvas.height = screenShareVideo.videoHeight;
    } else {
        commentCanvas.width = window.innerWidth;
        commentCanvas.height = window.innerHeight;
    }
    ctx.clearRect(0, 0, commentCanvas.width, commentCanvas.height);
    const currentTime = performance.now();
    for (let i = activeComments.length - 1; i >= 0; i--) {
        const comment = activeComments[i];
        const elapsed = currentTime - comment.startTime;
        if (elapsed < comment.duration) {
            const progress = elapsed / comment.duration;
            const startX = commentCanvas.width;
            const endX = -comment.width;
            comment.x = startX + (endX - startX) * progress;
            ctx.font = comment.font;
            ctx.fillStyle = comment.color;
            ctx.fillText(comment.text, comment.x, comment.y);
        } else {
            activeComments.splice(i, 1);
        }
    }
    requestAnimationFrame(animateCanvasComments);
}

// ===================== 画面共有 =====================
async function startScreenShare() {
    try {
        const stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
        screenShareVideo.srcObject = stream;
        screenShareVideo.play();
        uiContainer.style.display = 'none';
        startScreenShareButton.textContent = "画面共有中...";
        stream.getVideoTracks()[0].addEventListener('ended', () => {
            screenShareVideo.srcObject = null;
            ctx.clearRect(0, 0, commentCanvas.width, commentCanvas.height);
            uiContainer.style.display = 'block';
            startScreenShareButton.textContent = "画面共有を開始";
        });
        animateCanvasComments();
    } catch (err) {
        alert("画面共有の開始に失敗しました。ブラウザの許可を確認してください。");
        uiContainer.style.display = 'block';
        startScreenShareButton.textContent = "画面共有を開始";
    }
}

// ===================== イベントリスナー =====================
connectButton.addEventListener('click', () => {
    function attemptConnect() {
        const roomName = roomNameInput.value.trim();
        if (roomName === "") {
            roomNameInput.style.backgroundColor = 'orange';
            alert("部屋名を入力してください。");
            return;
        }
        roomNameInput.style.backgroundColor = '';
        connectWebSocket(roomName);
    }

    connectButton.addEventListener('click', attemptConnect);

    roomNameInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            attemptConnect();
        }
    });
});

startScreenShareButton.addEventListener('click', startScreenShare);

fontSizeSlider.addEventListener('input', () => {
    fontSizeDisplay.textContent = fontSizeSlider.value + 'px';
});

document.addEventListener('DOMContentLoaded', () => {
    commentCanvas.width = window.innerWidth;
    commentCanvas.height = window.innerHeight;
    fontSizeDisplay.textContent = fontSizeSlider.value + 'px';
    setConnectionUI('idle');
    try {
        const params = new URLSearchParams(window.location.search);
        const room = params.get('room');
        if (room) {
            const name = room.trim();
            if (name !== '') {
                roomNameInput.value = name;
                roomNameInput.style.backgroundColor = '';
                connectWebSocket(name);
            }
        }
    } catch (_) {
        // ignore
    }
});

// 入力が変更されたら、接続試行中の自動再接続を止める
roomNameInput.addEventListener('input', () => {
    if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
        reconnectTimeout = null;
    }
    // 入力変更時、接続中または接続試行中なら切断し、未接続状態に戻す
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
        try { ws.close(); } catch (_) {}
    }
    setConnectionUI('disconnected');
});

// ページ離脱時に接続を閉じる
window.addEventListener('beforeunload', () => {
    try { if (ws && ws.readyState === WebSocket.OPEN) ws.close(); } catch (_) {}
});
