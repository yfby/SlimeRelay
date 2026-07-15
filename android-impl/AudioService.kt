package com.github.yfby.slimerelay

import android.media.AudioFormat
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaRecorder
import kotlin.math.abs

class MicRecorder(private val onSamples: (FloatArray) -> Unit, private val onError: (String) -> Unit) {
    private var audioRecord: AudioRecord? = null
    @Volatile private var recording = false

    fun start() {
        val minBuf = AudioRecord.getMinBufferSize(
            SlimeRelayConstants.SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        val bufSize = maxOf(minBuf, SlimeRelayConstants.CHUNK_SAMPLES * 2 * 2)

        audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            SlimeRelayConstants.SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            bufSize
        )

        if (audioRecord?.state != AudioRecord.STATE_INITIALIZED) {
            onError("AudioRecord failed to initialize")
            return
        }

        recording = true
        audioRecord?.startRecording()

        Thread({
            val readBuf = ShortArray(SlimeRelayConstants.CHUNK_SAMPLES)
            while (recording) {
                val read = audioRecord?.read(readBuf, 0, readBuf.size) ?: -1
                if (read > 0) {
                    val floats = FloatArray(read) { i ->
                        readBuf[i].toFloat() / Short.MAX_VALUE.toFloat()
                    }
                    onSamples(floats)
                } else if (read < 0) {
                    onError("AudioRecord read error: $read")
                    break
                }
            }
        }, "MicRecorder").start()
    }

    fun stop() {
        recording = false
        audioRecord?.stop()
        audioRecord?.release()
        audioRecord = null
    }
}

class SpeakerPlayer(private val onError: (String) -> Unit) {
    private var audioTrack: AudioTrack? = null
    @Volatile private var playing = false

    fun start() {
        val minBuf = AudioTrack.getMinBufferSize(
            SlimeRelayConstants.SAMPLE_RATE,
            AudioFormat.CHANNEL_OUT_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        val bufSize = maxOf(minBuf, SlimeRelayConstants.CHUNK_SAMPLES * 2 * 4)

        audioTrack = AudioTrack.Builder()
            .setAudioFormat(
                AudioFormat.Builder()
                    .setSampleRate(SlimeRelayConstants.SAMPLE_RATE)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .build()
            )
            .setBufferSizeInBytes(bufSize)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()

        playing = true
        audioTrack?.play()
    }

    fun writeSamples(samples: FloatArray) {
        if (!playing) return
        val shorts = ShortArray(samples.size) { i ->
            (samples[i].coerceIn(-1f, 1f) * Short.MAX_VALUE).toInt().toShort()
        }
        audioTrack?.write(shorts, 0, shorts.size, AudioTrack.WRITE_NON_BLOCKING)
    }

    fun computeLevel(samples: FloatArray): Float {
        if (samples.isEmpty()) return 0f
        val sum = samples.sumOf { abs(it).toDouble() }
        return (sum / samples.size).coerceIn(0.0, 1.0).toFloat()
    }

    fun stop() {
        playing = false
        audioTrack?.stop()
        audioTrack?.release()
        audioTrack = null
    }
}
