import { Component, Show, createEffect, createMemo } from 'solid-js'

import './style/messages.scss'

import { createStore, produce } from 'solid-js/store'
import { httpx } from 'shared'
import { Message } from 'models'
import { useNavigate, useParams } from '@solidjs/router'
import { ChevronLeftIcon, ChevronRightIcon, EyeIcon } from 'icons'
import { COUNTRY_LIST } from './country-list'
import { SERVICE_LIST } from './service-list'

const Messages: Component = () => {
    type State = {
        messages: Message[]
        page: number
    }

    const [state, setState] = createStore<State>({ messages: [], page: 0 })
    const UP = useParams()
    const nav = useNavigate()

    createEffect(() => {
        let page = parseInt(UP.page || '0')
        if (isNaN(page) || page < 0) {
            nav('/messages/')
            return
        }
        fetch_messages(page)
    })

    function fetch_messages(page: number) {
        httpx({
            url: '/api/user/messages/?page=' + page,
            method: 'GET',
            type: 'json',
            onLoad(x) {
                if (x.status != 200) return

                setState({ messages: x.response, page })
            },
        })
    }

    function seen(id: number) {
        httpx({
            url: `/api/user/messages/${id}/seen/`,
            method: 'POST',
            type: 'json',
            onLoad(x) {
                if (x.status != 200) return

                setState(
                    produce(s => {
                        let idx = s.messages.findIndex(m => m.id == id)
                        if (idx != -1) {
                            s.messages[idx].seen = true
                        }
                    })
                )
            },
        })
    }

    return (
        <div class='messages-fnd'>
            <Show when={state.messages.length == 0}>
                <div class='message-empty'>
                    <h2>پیامی یافت نشد</h2>
                    <span>صفحه: {state.page}</span>
                </div>
            </Show>
            <div class='message-list'>
                {state.messages.map(m => (
                    <div class='message'>
                        <div class='info'>
                            <div class='row'>
                                <span class='code'>code: {m.code}</span>
                                <ServiceDpy d={m.service} />
                                <CountryDpy d={m.country} />
                            </div>
                            <textarea
                                class='text'
                                rows={m.text.split('\\n').length}
                                disabled
                                dir='auto'
                            >
                                {m.text.replaceAll('\\n', '\n')}
                            </textarea>
                            {/*<p class='text'>{m.text}</p>*/}
                            <span class='date'>
                                {new Date(m.timestamp * 1e3).toLocaleString()}
                            </span>
                            <span>{m.received_at}</span>
                        </div>
                        <Show when={!m.seen}>
                            <div class='actions'>
                                <button
                                    class='styled icon'
                                    onClick={() => seen(m.id)}
                                >
                                    <EyeIcon />
                                </button>
                            </div>
                        </Show>
                    </div>
                ))}
            </div>
            <div class='pagination'>
                <Show when={state.page > 0}>
                    <button
                        class='styled'
                        onClick={() => nav('/messages/' + (state.page - 1))}
                    >
                        <ChevronLeftIcon />
                    </button>
                </Show>
                <Show when={state.messages.length >= 32}>
                    <button
                        class='styled'
                        onClick={() => nav('/messages/' + (state.page + 1))}
                    >
                        <ChevronRightIcon />
                    </button>
                </Show>
            </div>
        </div>
    )
}

const CountryDpy: Component<{ d: string }> = P => {
    const country = createMemo(() => {
        return COUNTRY_LIST.find(c => c[0].toString() == P.d)
    })
    return (
        <Show when={country()}>
            <span>
                {country()[3]} - {country()[4]}
            </span>
        </Show>
    )
}

const ServiceDpy: Component<{ d: string }> = P => {
    const service = createMemo(() => {
        return SERVICE_LIST.find(s => s[0] == P.d)
    })
    return (
        <Show when={service()}>
            <span>{service()[2] || service()[1]}</span>
        </Show>
    )
}

export default Messages
