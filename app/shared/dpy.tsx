import { OrderStatus } from 'models'
import { COUNTRY_LIST } from './country-list'
import { SERVICE_LIST } from './service-list'
import { Component, Show, createMemo } from 'solid-js'

export const STATUS_TABLE: { [k in OrderStatus]: string } = {
    done: 'تکمیل',
    wating: 'درحال تکمیل',
    refunded: 'بازپرداخت شد',
}

export const CountryDpy: Component<{ d: string }> = P => {
    const country = createMemo(() => {
        return COUNTRY_LIST.find(c => c[0].toString() == P.d)
    })
    return (
        <Show when={country()}>
            <span>
                {country()[4]} {country()[3]}
            </span>
        </Show>
    )
}

export const ServiceDpy: Component<{ d: string }> = P => {
    const service = createMemo(() => {
        return SERVICE_LIST.find(s => s[0] == P.d)
    })
    return (
        <Show when={service()}>
            <span>{service()[2] || service()[1]}</span>
        </Show>
    )
}

export const TomanDpy: Component<{ irr: number }> = P => (
    <>{(~~(P.irr / 10)).toLocaleString()}</>
)
